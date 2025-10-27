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
    /// List of symbols to track (e.g., "BTC-USDT").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OkxMessage {
    pub arg: OkxArg,
    pub data: Vec<OkxTickerData>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OkxArg {
    pub channel: String,
    #[serde(rename = "instId")]
    pub inst_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OkxTickerData {
    #[serde(rename = "instType")]
    pub inst_type: String,
    #[serde(rename = "instId")]
    pub inst_id: String,
    pub last: String,
    #[serde(rename = "lastSz")]
    pub last_sz: String,
    #[serde(rename = "askPx")]
    pub ask_px: String,
    #[serde(rename = "bidPx")]
    pub bid_px: String,
    #[serde(rename = "open24h")]
    pub open_24h: String,
    #[serde(rename = "high24h")]
    pub high_24h: String,
    #[serde(rename = "low24h")]
    pub low_24h: String,
    #[serde(rename = "vol24h")]
    pub vol_24h: String,
    pub ts: String,
}

impl From<OkxTickerData> for crate::UnifiedTicker {
    fn from(data: OkxTickerData) -> Self {
        let timestamp = data.ts.parse().unwrap_or_else(|_| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });

        Self {
            venue: "okx".to_owned(),
            symbol: data.inst_id,
            timestamp,
            last_price: data.last.parse().unwrap_or(0.0),
            bid: data.bid_px.parse().ok(),
            ask: data.ask_px.parse().ok(),
            volume_24h: data.vol_24h.parse().ok(),
            high_24h: data.high_24h.parse().ok(),
            low_24h: data.low_24h.parse().ok(),
        }
    }
}

pub async fn connect(
    config: &OkxConfig,
) -> Result<impl futures::Stream<Item = Result<OkxTickerData, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to OKX: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to tickers channel for all configured symbols
    let args: Vec<_> = config
        .track_symbols
        .iter()
        .map(|symbol| {
            serde_json::json!({
                "channel": "tickers",
                "instId": symbol
            })
        })
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
            Ok(Message::Text(text)) => match serde_json::from_str::<OkxMessage>(&text) {
                Ok(message) if message.arg.channel == "tickers" => {
                    message.data.into_iter().next().map(Ok)
                }
                Ok(_) => None, // Skip subscription confirmations
                Err(_) => None,
            },
            Ok(_) => None,
            Err(e) => Some(Err(format!("WebSocket error: {}", e))),
        }
    }))
}

#[ignore]
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
