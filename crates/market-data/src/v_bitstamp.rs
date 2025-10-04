use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://ws.bitstamp.net";

// BTC/USD Requirements:
//
//   1. Endpoint: wss://ws.bitstamp.net (WebSocket API v2)
//   2. Symbol: btcusd (lowercase format)
//   3. Channel: live_trades_btcusd (real-time trade data)
//   4. Authentication: Not required for public market data
//   5. Connection: Standard WebSocket with event-based messaging
//
//   Subscription message:
//   {
//     "event": "bts:subscribe",
//     "data": {
//       "channel": "live_trades_btcusd"
//     }
//   }
//
//   Subscription confirmation:
//   {
//     "event": "bts:subscription_succeeded",
//     "channel": "live_trades_btcusd",
//     "data": {}
//   }
//
//   Trade data payload structure:
//   {
//     "data": {
//       "id": 21565524,                  // Trade ID
//       "timestamp": "1505558814",       // Unix timestamp
//       "amount": 0.01513062,            // Trade amount in BTC
//       "amount_str": "0.01513062",      // Trade amount as string
//       "price": 42280.5,               // CURRENT PRICE (trade price)
//       "price_str": "42280.50",         // Trade price as string
//       "type": 1,                       // Order type (0=buy, 1=sell)
//       "cost": 639.67,                  // Trade cost (amount * price)
//       "buy_order_id": 297260696,       // Buy order ID
//       "sell_order_id": 297260910       // Sell order ID
//     },
//     "event": "trade",
//     "channel": "live_trades_btcusd"
//   }
//
//   Alternative channels available:
//   - order_book_btcusd: Real-time order book updates
//   - detail_order_book_btcusd: Detailed order book with full depth
//   - diff_order_book_btcusd: Incremental order book changes

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitstampConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

#[tokio::test]
async fn test_bitstamp_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for BTC/USD live trades
    let subscribe_msg = r#"{
        "event": "bts:subscribe",
        "data": {
            "channel": "live_trades_btcusd"
        }
    }"#;

    stream
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .expect("Failed to send subscription");

    // Read subscription confirmation and first trade message
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    panic!();
}
