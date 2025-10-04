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
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

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
