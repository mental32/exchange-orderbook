use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://api-pub.bitfinex.com/ws/2";

// BTC/USD Requirements:
//
//   1. Endpoint: wss://api-pub.bitfinex.com/ws/2 (public channels)
//   2. Symbol: tBTCUSD (trading pairs prefixed with 't')
//   3. Channel: ticker (high-level market overview)
//   4. Connection limits: 20 connections per minute for public channels
//
//   Subscription message:
//   {
//     "event": "subscribe",
//     "channel": "ticker",
//     "symbol": "tBTCUSD"
//   }
//
//   Subscription confirmation:
//   {
//     "event": "subscribed",
//     "channel": "ticker",
//     "chanId": 224555,
//     "symbol": "tBTCUSD",
//     "pair": "BTCUSD"
//   }
//
//   Ticker data payload structure:
//   [
//     CHANNEL_ID,
//     [
//       BID,                    // Best bid price
//       BID_SIZE,               // Best bid quantity
//       ASK,                    // Best ask price
//       ASK_SIZE,               // Best ask quantity
//       DAILY_CHANGE,           // Absolute price change
//       DAILY_CHANGE_RELATIVE,  // Relative price change (%)
//       LAST_PRICE,             // CURRENT PRICE (last traded price)
//       VOLUME,                 // Daily volume
//       HIGH,                   // 24h high price
//       LOW                     // 24h low price
//     ]
//   ]
//
//   Example response:
//   [
//     443378,
//     [
//       42243.5,  // Bid
//       31.89,    // Bid Size
//       42244.5,  // Ask
//       43.36,    // Ask Size
//       -550.8,   // Daily Change
//       -0.0674,  // Daily Change %
//       42244.1,  // Last Price (CURRENT PRICE)
//       8314.71,  // Volume
//       43257.8,  // High
//       41500.0   // Low
//     ]
//   ]

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitfinexConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

#[tokio::test]
async fn test_bitfinex_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for tBTCUSD ticker
    let subscribe_msg = r#"{
        "event": "subscribe",
        "channel": "ticker",
        "symbol": "tBTCUSD"
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
