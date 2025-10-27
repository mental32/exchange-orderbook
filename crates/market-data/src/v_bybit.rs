use futures::SinkExt as _;
use futures::StreamExt as _;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://stream.bybit.com/v5/public/spot";

// BTC/USDT Requirements:
//
//   1. Endpoint: wss://stream.bybit.com/v5/public/spot (spot trading)
//   2. Symbol: BTCUSDT (no separator, USD refers to USDT)
//   3. Topic: tickers.BTCUSDT
//   4. Push frequency: Real-time for spot trading
//   5. Heartbeat: Send "ping" every 20 seconds to maintain connection
//
//   Subscription message:
//   {
//     "op": "subscribe",
//     "args": ["tickers.BTCUSDT"]
//   }
//
//   Subscription confirmation:
//   {
//     "success": true,
//     "ret_msg": "",
//     "conn_id": "aa01fbf0-5649-4cc9-80cb-2c9638e0b37e",
//     "op": "subscribe"
//   }
//
//   Ticker data payload structure:
//   {
//     "topic": "tickers.BTCUSDT",
//     "ts": 1673853746003,
//     "type": "snapshot",
//     "cs": 2588407389,
//     "data": {
//       "symbol": "BTCUSDT",
//       "lastPrice": "42109.77",        // CURRENT PRICE (last traded price)
//       "highPrice24h": "42426.99",     // 24h high price
//       "lowPrice24h": "41575.00",      // 24h low price
//       "prevPrice24h": "41704.93",     // Previous day's close price
//       "volume24h": "6780.866843",     // 24h trading volume
//       "turnover24h": "141946527.22",  // 24h turnover in USDT
//       "price24hPcnt": "0.0196",       // 24h price change percentage
//       "usdIndexPrice": "42120.24",    // USD index price
//       "bid1Price": "42109.50",        // Best bid price
//       "bid1Size": "1.234",            // Best bid quantity
//       "ask1Price": "42110.00",        // Best ask price
//       "ask1Size": "2.567"             // Best ask quantity
//     }
//   }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BybitConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track (e.g., "BTCUSDT").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BybitMessage {
    pub topic: String,
    pub ts: u64,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: BybitTickerData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BybitTickerData {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "highPrice24h")]
    pub high_price_24h: String,
    #[serde(rename = "lowPrice24h")]
    pub low_price_24h: String,
    #[serde(rename = "volume24h")]
    pub volume_24h: String,
    #[serde(rename = "bid1Price")]
    pub bid1_price: String,
    #[serde(rename = "ask1Price")]
    pub ask1_price: String,
}

impl From<BybitMessage> for crate::UnifiedTicker {
    fn from(msg: BybitMessage) -> Self {
        Self {
            venue: "bybit".to_owned(),
            symbol: msg.data.symbol,
            timestamp: msg.ts,
            last_price: msg.data.last_price.parse().unwrap_or(0.0),
            bid: msg.data.bid1_price.parse().ok(),
            ask: msg.data.ask1_price.parse().ok(),
            volume_24h: msg.data.volume_24h.parse().ok(),
            high_24h: msg.data.high_price_24h.parse().ok(),
            low_24h: msg.data.low_price_24h.parse().ok(),
        }
    }
}

pub async fn connect(
    config: &BybitConfig,
) -> Result<impl futures::Stream<Item = Result<BybitMessage, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Bybit: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to tickers for all configured symbols
    let args: Vec<String> = config
        .track_symbols
        .iter()
        .map(|s| format!("tickers.{}", s))
        .collect();

    let subscribe_msg = serde_json::json!({
        "op": "subscribe",
        "args": args
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .map_err(|e| format!("Failed to send subscription: {}", e))?;

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<BybitMessage>(&text) {
                    Ok(message) => Some(Ok(message)),
                    Err(_) => None, // Skip subscription confirmations and other messages
                }
            }
            Ok(_) => None,
            Err(e) => Some(Err(format!("WebSocket error: {}", e))),
        }
    }))
}

#[ignore]
#[tokio::test]
async fn test_bybit_connection() {
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");
    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());

    // Send subscription message for BTCUSDT ticker
    let subscribe_msg = r#"{
        "op": "subscribe",
        "args": ["tickers.BTCUSDT"]
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
