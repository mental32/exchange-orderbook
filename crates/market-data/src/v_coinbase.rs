use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://advanced-trade-ws.coinbase.com";

// BTC/USD Requirements:
//
//   1. Endpoint: wss://advanced-trade-ws.coinbase.com (market data)
//   2. Symbol: BTC-USD (dash format)
//   3. Channel: ticker (real-time price updates)
//   4. Authentication: Not required for public ticker data
//   5. Heartbeat: Recommended to subscribe to heartbeats channel
//
//   Subscription message:
//   {
//     "type": "subscribe",
//     "product_ids": ["BTC-USD"],
//     "channel": "ticker"
//   }
//
//   Subscription confirmation:
//   {
//     "type": "subscriptions",
//     "channels": [
//       {
//         "name": "ticker",
//         "product_ids": ["BTC-USD"]
//       }
//     ]
//   }
//
//   Ticker data payload structure:
//   {
//     "channel": "ticker",
//     "client_id": "",
//     "timestamp": "2023-02-09T20:30:37.167359596Z",
//     "sequence_num": 0,
//     "events": [
//       {
//         "type": "snapshot",
//         "tickers": [
//           {
//             "type": "ticker",
//             "product_id": "BTC-USD",
//             "price": "42932.98",           // CURRENT PRICE (last traded price)
//             "volume_24_h": "16038.28770938", // 24h trading volume
//             "low_24_h": "41835.29",        // 24h low price
//             "high_24_h": "43011.18",       // 24h high price
//             "low_52_w": "15460",           // 52-week low
//             "high_52_w": "48240",          // 52-week high
//             "price_percent_chg_24_h": "-4.15775596190603", // 24h price change %
//             "best_bid": "42932.50",        // Best bid price
//             "best_bid_quantity": "1.5",    // Best bid quantity
//             "best_ask": "42933.00",        // Best ask price
//             "best_ask_quantity": "2.1"     // Best ask quantity
//           }
//         ]
//       }
//     ]
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoinbaseConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track (e.g., "BTC-USD").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoinbaseMessage {
    pub channel: String,
    pub timestamp: String,
    pub events: Vec<CoinbaseEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoinbaseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub tickers: Vec<CoinbaseTicker>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoinbaseTicker {
    pub product_id: String,
    pub price: String,
    pub volume_24_h: String,
    pub low_24_h: String,
    pub high_24_h: String,
    pub best_bid: String,
    pub best_ask: String,
}

impl From<CoinbaseTicker> for crate::UnifiedTicker {
    fn from(ticker: CoinbaseTicker) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            venue: "coinbase".to_owned(),
            symbol: ticker.product_id,
            timestamp,
            last_price: ticker.price.parse().unwrap_or(0.0),
            bid: ticker.best_bid.parse().ok(),
            ask: ticker.best_ask.parse().ok(),
            volume_24h: ticker.volume_24_h.parse().ok(),
            high_24h: ticker.high_24_h.parse().ok(),
            low_24h: ticker.low_24_h.parse().ok(),
        }
    }
}

pub async fn connect(
    config: &CoinbaseConfig,
) -> Result<impl futures::Stream<Item = Result<CoinbaseTicker, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Coinbase: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to ticker channel for all configured symbols
    let subscribe_msg = serde_json::json!({
        "type": "subscribe",
        "product_ids": config.track_symbols,
        "channel": "ticker"
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription: {}", e))?;

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<CoinbaseMessage>(&text) {
                    Ok(message) if message.channel == "ticker" => {
                        // Flatten tickers from all events
                        let tickers: Vec<_> =
                            message.events.into_iter().flat_map(|e| e.tickers).collect();
                        tickers.into_iter().next().map(Ok)
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
async fn test_coinbase_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for BTC-USD ticker
    let subscribe_msg = r#"{
        "type": "subscribe",
        "product_ids": ["BTC-USD"],
        "channel": "ticker"
    }"#;

    stream
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .expect("Failed to send subscription");

    // Read subscription confirmation and first ticker message
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    panic!();
}
