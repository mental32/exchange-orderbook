use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://ws.okx.com:8443/ws/v5/public";

// BTC/USDT Requirements:
//
//   1. Endpoint: wss://ws.okx.com:8443/ws/v5/public (public channels)
//   2. Symbol: BTC-USDT (dash format)
//   3. Channel: tickers
//   4. Update frequency: 100ms if price changes, otherwise every second
//   5. Authentication: Not required for public ticker data
//
//   Subscription message:
//   {
//     "op": "subscribe",
//     "args": [
//       {
//         "channel": "tickers",
//         "instId": "BTC-USDT"
//       }
//     ]
//   }
//
//   Subscription confirmation:
//   {
//     "event": "subscribe",
//     "arg": {
//       "channel": "tickers",
//       "instId": "BTC-USDT"
//     },
//     "connId": "a4d3ae55"
//   }
//
//   Ticker data payload structure:
//   {
//     "arg": {
//       "channel": "tickers",
//       "instId": "BTC-USDT"
//     },
//     "data": [
//       {
//         "instType": "SPOT",             // Instrument type
//         "instId": "BTC-USDT",           // Trading pair
//         "last": "42352.5",              // CURRENT PRICE (last traded price)
//         "lastSz": "0.01734077",         // Last traded quantity
//         "askPx": "42352.5",             // Best ask price
//         "askSz": "1.26608653",          // Best ask quantity
//         "bidPx": "42352.4",             // Best bid price
//         "bidSz": "0.46033212",          // Best bid quantity
//         "open24h": "42463.6",           // Opening price 24h ago
//         "high24h": "42722.5",           // 24h high price
//         "low24h": "42211.0",            // 24h low price
//         "vol24h": "5108.83404369",      // 24h volume in base currency (BTC)
//         "volCcy24h": "119705529.10",    // 24h volume in quote currency (USDT)
//         "sodUtc0": "42431.3",           // Price at 00:00 UTC today
//         "sodUtc8": "42603.6",           // Price at 08:00 UTC today
//         "ts": "1675510037012"           // Timestamp in milliseconds
//       }
//     ]
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OkxConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

#[tokio::test]
async fn test_okx_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for BTC-USDT ticker
    let subscribe_msg = r#"{
        "op": "subscribe",
        "args": [
            {
                "channel": "tickers",
                "instId": "BTC-USDT"
            }
        ]
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
