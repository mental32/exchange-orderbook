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
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

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
