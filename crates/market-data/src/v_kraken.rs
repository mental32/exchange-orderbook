use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://ws.kraken.com/v2";

// BTC/USD Requirements:
//
//   1. Endpoint: wss://ws.kraken.com/v2
//   2. Symbol: BTC/USD (ISO 4217-A3 format)
//   3. Channel: ticker (level 1 market data)
//   4. Connection: Requires TLS with SNI
//
//   Subscription message:
//   {
//     "method": "subscribe",
//     "params": {
//       "channel": "ticker",
//       "symbol": ["BTC/USD"]
//     }
//   }
//
//   Response payload structure:
//   {
//     "channel": "ticker",
//     "type": "snapshot",
//     "data": [{
//       "symbol": "BTC/USD",
//       "bid": 42243.10,
//       "bid_qty": 3.0,
//       "ask": 42243.20,
//       "ask_qty": 1.5,
//       "last": 42243.20,         // CURRENT PRICE
//       "volume": 634.15053067,
//       "vwap": 41908.86644,
//       "low": 40772.20000,
//       "high": 42474.30000,
//       "change": -0.17,
//       "change_pct": -0.17
//     }]
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KrakenConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track (e.g., "BTC/USD").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KrakenMessage {
    pub channel: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: Vec<KrakenTickerData>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KrakenTickerData {
    pub symbol: String,
    pub bid: f64,
    pub bid_qty: f64,
    pub ask: f64,
    pub ask_qty: f64,
    pub last: f64,
    pub volume: f64,
    pub vwap: f64,
    pub low: f64,
    pub high: f64,
    pub change: f64,
    pub change_pct: f64,
}

impl From<KrakenTickerData> for crate::UnifiedTicker {
    fn from(data: KrakenTickerData) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            venue: "kraken".to_owned(),
            symbol: data.symbol,
            timestamp,
            last_price: data.last,
            bid: Some(data.bid),
            ask: Some(data.ask),
            volume_24h: Some(data.volume),
            high_24h: Some(data.high),
            low_24h: Some(data.low),
        }
    }
}

pub async fn connect(
    config: &KrakenConfig,
) -> Result<impl futures::Stream<Item = Result<KrakenTickerData, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Kraken: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to ticker channel for all configured symbols
    let subscribe_msg = serde_json::json!({
        "method": "subscribe",
        "params": {
            "channel": "ticker",
            "symbol": config.track_symbols
        }
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription: {}", e))?;

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<KrakenMessage>(&text) {
                    Ok(message) if message.channel == "ticker" => {
                        message.data.into_iter().next().map(Ok)
                    }
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
async fn test_kraken_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for BTC/USD ticker
    let subscribe_msg = r#"{
        "method": "subscribe",
        "params": {
            "channel": "ticker",
            "symbol": ["BTC/USD"]
        }
    }"#;

    stream
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .expect("Failed to send subscription");

    // Read subscription confirmation and first ticker message
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    panic!();
}
