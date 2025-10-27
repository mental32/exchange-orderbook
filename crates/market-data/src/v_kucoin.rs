use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_REST_API_BASE: &str = "https://api.kucoin.com";
pub const PUBLIC_TOKEN_ENDPOINT: &str = "/api/v1/bullet-public";

// BTC/USDT Requirements:
//
//   1. First get token: POST https://api.kucoin.com/api/v1/bullet-public
//   2. Use returned WebSocket endpoint from instanceServers
//   3. Symbol: BTC-USDT (dash format)
//   4. Topic: /market/ticker:BTC-USDT
//   5. Connection: Requires token in URL query parameter
//
//   Token request: POST /api/v1/bullet-public
//   Response includes:
//   {
//     "data": {
//       "token": "2neAiuYvAU61ZDXANAGAsiL4-iAExhsBXZxftpOeh_55i3Ysy2q2LEsEWU64mdzUOPusi34M_wGoSf7iNyEWJ1UQy1jh_jYnHRvQoaq-b18t6JqRzByfjOPqRmm1dXOq5RiZuylQMqGrvjvq4lQQAAAA3I0EAAA=",
//       "instanceServers": [
//         {
//           "endpoint": "wss://ws-api-spot.kucoin.com/",
//           "encrypt": true,
//           "protocol": "websocket",
//           "pingInterval": 18000,
//           "pingTimeout": 10000
//         }
//       ]
//     }
//   }
//
//   Subscription message:
//   {
//     "id": 1545910660739,
//     "type": "subscribe",
//     "topic": "/market/ticker:BTC-USDT",
//     "privateChannel": false,
//     "response": true
//   }
//
//   Response payload structure:
//   {
//     "type": "message",
//     "topic": "/market/ticker:BTC-USDT",
//     "subject": "trade.ticker",
//     "data": {
//       "sequence": "1545896668986",
//       "price": "0.08",        // CURRENT PRICE (last traded price)
//       "size": "0.011",        // Last traded amount
//       "bestAsk": "0.08",      // Best ask price
//       "bestAskSize": "0.18",  // Best ask size
//       "bestBid": "0.049",     // Best bid price
//       "bestBidSize": "0.036", // Best bid size
//       "Time": 1704873323416   // Latest transaction matching time
//     }
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KucoinConfig {
    /// REST API base URL for getting tokens.
    pub rest_api_base: String,
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    data: TokenData,
}

#[derive(Debug, Deserialize)]
struct TokenData {
    token: String,
    #[serde(rename = "instanceServers")]
    instance_servers: Vec<InstanceServer>,
}

#[derive(Debug, Deserialize)]
struct InstanceServer {
    endpoint: String,
    #[serde(rename = "pingInterval")]
    ping_interval: u64,
    #[serde(rename = "pingTimeout")]
    ping_timeout: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KucoinMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub topic: String,
    pub subject: String,
    pub data: KucoinTickerData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KucoinTickerData {
    pub sequence: String,
    pub price: String,
    pub size: String,
    #[serde(rename = "bestAsk")]
    pub best_ask: String,
    #[serde(rename = "bestAskSize")]
    pub best_ask_size: String,
    #[serde(rename = "bestBid")]
    pub best_bid: String,
    #[serde(rename = "bestBidSize")]
    pub best_bid_size: String,
}

impl From<KucoinMessage> for crate::UnifiedTicker {
    fn from(msg: KucoinMessage) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Extract symbol from topic like "/market/ticker:BTC-USDT"
        let symbol = msg.topic.split(':').nth(1).unwrap_or("UNKNOWN").to_string();

        Self {
            venue: "kucoin".to_owned(),
            symbol,
            timestamp,
            last_price: msg.data.price.parse().unwrap_or(0.0),
            bid: msg.data.best_bid.parse().ok(),
            ask: msg.data.best_ask.parse().ok(),
            volume_24h: None,
            high_24h: None,
            low_24h: None,
        }
    }
}

pub async fn connect(
    config: &KucoinConfig,
) -> Result<impl futures::Stream<Item = Result<KucoinMessage, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    // Step 1: Get public token via REST API
    let client = reqwest::Client::new();
    let token_url = format!("{}{}", config.rest_api_base, PUBLIC_TOKEN_ENDPOINT);

    let response = client
        .post(&token_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get Kucoin token: {}", e))?;

    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Kucoin token response: {}", e))?;

    let token = token_response.data.token;
    let endpoint = &token_response.data.instance_servers[0].endpoint;

    // Step 2: Connect to WebSocket with token
    let ws_url = format!("{}?token={}", endpoint, token);

    let (stream, _response) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| format!("Failed to connect to Kucoin WebSocket: {}", e))?;

    let (mut write, read) = stream.split();

    // Step 3: Subscribe to ticker for all configured symbols
    for symbol in &config.track_symbols {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let subscribe_msg = serde_json::json!({
            "id": timestamp,
            "type": "subscribe",
            "topic": format!("/market/ticker:{}", symbol),
            "privateChannel": false,
            "response": true
        });

        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| format!("Failed to send subscription: {}", e))?;
    }

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<KucoinMessage>(&text) {
                    Ok(message) if message.msg_type == "message" => Some(Ok(message)),
                    Ok(_) => None, // Skip subscription confirmations
                    Err(_) => None,
                }
            }
            Ok(_) => None,
            Err(e) => Some(Err(format!("WebSocket error: {}", e))),
        }
    }))
}

#[ignore]
#[tokio::test]
async fn test_kucoin_connection() {
    use tokio_tungstenite::tungstenite::Message;

    // Step 1: Get public token
    let client = reqwest::Client::new();
    let token_url = format!("{}{}", DEFAULT_REST_API_BASE, PUBLIC_TOKEN_ENDPOINT);

    let response = client
        .post(&token_url)
        .send()
        .await
        .expect("Failed to get token");

    let token_response: TokenResponse = response
        .json()
        .await
        .expect("Failed to parse token response");

    let token = token_response.data.token;
    let endpoint = &token_response.data.instance_servers[0].endpoint;

    // Step 2: Connect to WebSocket with token
    let ws_url = format!("{}?token={}&[connectId={}]", endpoint, token, "test123");

    let (mut stream, response) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Step 3: Send subscription message for BTC-USDT ticker
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let subscribe_msg = format!(
        r#"{{
        "id": {},
        "type": "subscribe",
        "topic": "/market/ticker:BTC-USDT",
        "privateChannel": false,
        "response": true
    }}"#,
        timestamp
    );

    stream
        .send(Message::Text(subscribe_msg))
        .await
        .expect("Failed to send subscription");

    // Step 4: Read subscription confirmation and first ticker message
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    panic!();
}
