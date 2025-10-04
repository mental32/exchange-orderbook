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
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

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
