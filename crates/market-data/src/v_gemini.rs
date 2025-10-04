use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://api.gemini.com/v1/marketdata/BTCUSD";

// BTC/USD Requirements:
//
//   1. Endpoint: wss://api.gemini.com/v1/marketdata/BTCUSD
//   2. Symbol: BTCUSD (uppercase, no separator)
//   3. Connection: Direct stream, no subscription message needed
//   4. Type: Public API, no authentication required
//
//   Response payload structure (various event types):
//
//   Initial snapshot:
//   {
//     "type": "update",
//     "eventId": 36902233362,
//     "socket_sequence": 661,
//     "events": [
//       {
//         "type": "change",
//         "side": "bid",
//         "price": "42243.20",
//         "remaining": "1.5",        // Order book quantity
//         "delta": "1.5",
//         "reason": "place"
//       }
//     ]
//   }
//
//   Trade events:
//   {
//     "type": "update",
//     "eventId": 36902233363,
//     "socket_sequence": 662,
//     "events": [
//       {
//         "type": "trade",
//         "price": "42243.20",       // CURRENT PRICE (trade price)
//         "quantity": "0.5",         // Trade quantity
//         "side": "buy"              // Aggressor side
//       }
//     ]
//   }
//
//   Order book changes:
//   {
//     "type": "update",
//     "eventId": 36902233364,
//     "socket_sequence": 663,
//     "events": [
//       {
//         "type": "change",
//         "side": "ask",             // bid or ask
//         "price": "42243.30",       // Price level
//         "remaining": "0",          // New quantity (0 = removed)
//         "delta": "-2.1",           // Change in quantity
//         "reason": "cancel"         // place, cancel, trade
//       }
//     ]
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeminiConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}

#[tokio::test]
async fn test_gemini_connection() {
    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Gemini streams immediately upon connection - no subscription message needed
    // Read initial market data updates
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    dbg!(stream.next().await);
    panic!();
}
