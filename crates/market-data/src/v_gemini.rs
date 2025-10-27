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
    /// List of symbols to track (e.g., "BTCUSD").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeminiMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "eventId")]
    pub event_id: u64,
    pub events: Vec<GeminiEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum GeminiEvent {
    #[serde(rename = "trade")]
    Trade {
        price: String,
        quantity: String,
        side: String,
    },
    #[serde(rename = "change")]
    Change {
        side: String,
        price: String,
        remaining: String,
        delta: String,
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct GeminiTrade {
    pub price: f64,
    pub quantity: f64,
    pub side: String,
}

impl From<GeminiTrade> for crate::UnifiedTicker {
    fn from(trade: GeminiTrade) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            venue: "gemini".to_owned(),
            symbol: "BTCUSD".to_owned(),
            timestamp,
            last_price: trade.price,
            bid: None,
            ask: None,
            volume_24h: None,
            high_24h: None,
            low_24h: None,
        }
    }
}

pub async fn connect(
    config: &GeminiConfig,
) -> Result<impl futures::Stream<Item = Result<GeminiTrade, String>>, String> {
    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Gemini: {}", e))?;

    let (_write, read) = stream.split();

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                match serde_json::from_str::<GeminiMessage>(&text) {
                    Ok(message) if message.msg_type == "update" => {
                        // Extract trade events
                        for event in message.events {
                            if let GeminiEvent::Trade {
                                price,
                                quantity,
                                side,
                            } = event
                            {
                                if let (Ok(p), Ok(q)) = (price.parse(), quantity.parse()) {
                                    return Some(Ok(GeminiTrade {
                                        price: p,
                                        quantity: q,
                                        side,
                                    }));
                                }
                            }
                        }
                        None
                    }
                    Ok(_) => None,
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
