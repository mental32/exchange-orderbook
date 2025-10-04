use futures::StreamExt as _;

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
