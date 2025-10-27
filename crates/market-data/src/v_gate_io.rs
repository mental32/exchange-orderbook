use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://api.gateio.ws/ws/v4/";

// BTC/USDT Requirements:
//
//   1. Endpoint: wss://api.gateio.ws/ws/v4/
//   2. Symbol: BTC_USDT (underscore format)
//   3. Channel: spot.tickers
//   4. Update frequency: 1000ms (1 second)
//
//   Subscription message:
//   {
//     "time": <current_timestamp>,
//     "channel": "spot.tickers",
//     "event": "subscribe",
//     "payload": ["BTC_USDT"]
//   }
//
//   Response payload structure:
//   {
//     "result": {
//       "currency_pair": "BTC_USDT",
//       "last": "42243.20",           // CURRENT PRICE
//       "lowest_ask": "42243.20",     // Best ask price
//       "highest_bid": "42243.10",    // Best bid price
//       "change_percentage": "-0.17", // Price change percentage
//       "base_volume": "634.15053067", // Base trading volume
//       "quote_volume": "26789123.45" // Quote trading volume
//     }
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GateIoConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track (e.g., "BTC_USDT").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GateIoMessage {
    pub time: u64,
    pub channel: String,
    pub event: String,
    #[serde(default)]
    pub result: Option<GateIoTickerResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GateIoTickerResult {
    pub currency_pair: String,
    pub last: String,
    pub lowest_ask: String,
    pub highest_bid: String,
    pub change_percentage: String,
    pub base_volume: String,
    pub quote_volume: String,
}

impl From<GateIoTickerResult> for crate::UnifiedTicker {
    fn from(result: GateIoTickerResult) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            venue: "gate_io".to_owned(),
            symbol: result.currency_pair,
            timestamp,
            last_price: result.last.parse().unwrap_or(0.0),
            bid: result.highest_bid.parse().ok(),
            ask: result.lowest_ask.parse().ok(),
            volume_24h: result.base_volume.parse().ok(),
            high_24h: None,
            low_24h: None,
        }
    }
}

pub async fn connect(
    config: &GateIoConfig,
) -> Result<impl futures::Stream<Item = Result<GateIoTickerResult, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Gate.io: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to spot.tickers channel for all configured symbols
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let subscribe_msg = serde_json::json!({
        "time": timestamp,
        "channel": "spot.tickers",
        "event": "subscribe",
        "payload": config.track_symbols
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription: {}", e))?;

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<GateIoMessage>(&text) {
                    Ok(message) if message.event == "update" => message.result.map(Ok),
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
async fn test_gate_io_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for BTC_USDT ticker
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let subscribe_msg = format!(
        r#"{{
        "time": {},
        "channel": "spot.tickers",
        "event": "subscribe",
        "payload": ["BTC_USDT"]
    }}"#,
        timestamp
    );

    stream
        .send(Message::Text(subscribe_msg))
        .await
        .expect("Failed to send subscription");

    // Read subscription confirmation and first ticker message
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    panic!();
}
