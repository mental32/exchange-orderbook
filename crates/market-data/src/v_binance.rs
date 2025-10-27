use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://stream.binance.com:9443/ws/btcusdt@ticker";

// BTC/USD Requirements:
//
//   1. Endpoint: wss://stream.binance.com:9443/ws/btcusdt@ticker
//   2. Stream: btcusdt@ticker (note: lowercase symbol)
//   3. Update frequency: 1000ms (1 second)
//   4. Connection management: Must respond to ping frames with pong
//
//   Payload structure for BTC/USDT ticker:
//   {
//     "e": "24hrTicker",     // Event type
//     "E": 1672515782136,    // Event time (milliseconds)
//     "s": "BTCUSDT",        // Symbol
//     "p": "0.0015",         // Price change
//     "P": "250.00",         // Price change percent
//     "w": "0.0018",         // Weighted average price
//     "c": "0.0025",         // Last price (CURRENT PRICE)
//     "Q": "10",             // Last quantity
//     "o": "0.0010",         // Open price
//     "h": "0.0025",         // High price
//     "l": "0.0010",         // Low price
//     "v": "10000",          // Base asset volume
//     "q": "18",             // Quote asset volume
//     "O": 0,                // Open time
//     "C": 86400000,         // Close time
//     "F": 0,                // First trade ID
//     "L": 18150,            // Last trade ID
//     "n": 18151             // Trade count
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BinanceConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track (e.g., "btcusdt").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BinanceTickerEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub last_price: String,
    #[serde(rename = "h")]
    pub high_price: String,
    #[serde(rename = "l")]
    pub low_price: String,
    #[serde(rename = "v")]
    pub base_volume: String,
}

impl From<BinanceTickerEvent> for crate::UnifiedTicker {
    fn from(event: BinanceTickerEvent) -> Self {
        Self {
            venue: "binance".to_owned(),
            symbol: event.symbol,
            timestamp: event.event_time,
            last_price: event.last_price.parse().unwrap_or(0.0),
            bid: None,
            ask: None,
            volume_24h: event.base_volume.parse().ok(),
            high_24h: event.high_price.parse().ok(),
            low_24h: event.low_price.parse().ok(),
        }
    }
}

pub async fn connect(
    config: &BinanceConfig,
) -> Result<impl futures::Stream<Item = Result<BinanceTickerEvent, String>>, String> {
    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Binance: {}", e))?;

    let (_write, read) = stream.split();

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                serde_json::from_str::<BinanceTickerEvent>(&text)
                    .map_err(|e| format!("Failed to parse Binance message: {}", e))
                    .ok()
                    .map(Ok)
            }
            Ok(_) => None,
            Err(e) => Some(Err(format!("WebSocket error: {}", e))),
        }
    }))
}

#[ignore]
#[tokio::test]
async fn test_binance_connection() {
    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());
    dbg!(stream.next().await);
    panic!();
}
