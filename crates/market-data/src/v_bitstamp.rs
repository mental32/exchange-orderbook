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
    /// List of symbols to track (e.g., "btcusd").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitstampMessage {
    pub event: String,
    pub channel: String,
    #[serde(default)]
    pub data: Option<BitstampTradeData>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitstampTradeData {
    pub id: u64,
    pub timestamp: String,
    pub amount: f64,
    pub amount_str: String,
    pub price: f64,
    pub price_str: String,
    #[serde(rename = "type")]
    pub trade_type: u8,
}

impl From<BitstampTradeData> for crate::UnifiedTicker {
    fn from(trade: BitstampTradeData) -> Self {
        let timestamp = trade.timestamp.parse::<u64>().unwrap_or_else(|_| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }) * 1000;

        Self {
            venue: "bitstamp".to_owned(),
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
    config: &BitstampConfig,
) -> Result<impl futures::Stream<Item = Result<BitstampTradeData, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Bitstamp: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to all configured symbols
    for symbol in &config.track_symbols {
        let subscribe_msg = serde_json::json!({
            "event": "bts:subscribe",
            "data": {
                "channel": format!("live_trades_{}", symbol)
            }
        });
        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| format!("Failed to send subscription: {}", e))?;
    }

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<BitstampMessage>(&text) {
                    Ok(message) if message.event == "trade" => message.data.map(Ok),
                    Ok(_) => None, // Skip other events like subscription confirmations
                    Err(e) => Some(Err(format!("Failed to parse Bitstamp message: {}", e))),
                }
            }
            Ok(_) => None,
            Err(e) => Some(Err(format!("WebSocket error: {}", e))),
        }
    }))
}

#[ignore]
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
