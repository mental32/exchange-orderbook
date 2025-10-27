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
    /// List of symbols to track (e.g., "tBTCUSD").
    pub track_symbols: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BitfinexMessage {
    Event(BitfinexEvent),
    Ticker(BitfinexTickerData),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitfinexEvent {
    pub event: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default, rename = "chanId")]
    pub chan_id: Option<u64>,
    #[serde(default)]
    pub symbol: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BitfinexTickerData(pub u64, pub Vec<f64>);

#[derive(Clone, Debug)]
pub struct BitfinexTicker {
    pub channel_id: u64,
    pub bid: f64,
    pub bid_size: f64,
    pub ask: f64,
    pub ask_size: f64,
    pub daily_change: f64,
    pub daily_change_relative: f64,
    pub last_price: f64,
    pub volume: f64,
    pub high: f64,
    pub low: f64,
}

impl TryFrom<BitfinexTickerData> for BitfinexTicker {
    type Error = String;

    fn try_from(data: BitfinexTickerData) -> Result<Self, Self::Error> {
        if data.1.len() < 10 {
            return Err("Invalid ticker data length".to_owned());
        }
        Ok(Self {
            channel_id: data.0,
            bid: data.1[0],
            bid_size: data.1[1],
            ask: data.1[2],
            ask_size: data.1[3],
            daily_change: data.1[4],
            daily_change_relative: data.1[5],
            last_price: data.1[6],
            volume: data.1[7],
            high: data.1[8],
            low: data.1[9],
        })
    }
}

impl From<BitfinexTicker> for crate::UnifiedTicker {
    fn from(ticker: BitfinexTicker) -> Self {
        Self {
            venue: "bitfinex".to_owned(),
            symbol: "BTCUSD".to_owned(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            last_price: ticker.last_price,
            bid: Some(ticker.bid),
            ask: Some(ticker.ask),
            volume_24h: Some(ticker.volume),
            high_24h: Some(ticker.high),
            low_24h: Some(ticker.low),
        }
    }
}

pub async fn connect(
    config: &BitfinexConfig,
) -> Result<impl futures::Stream<Item = Result<BitfinexTicker, String>>, String> {
    use futures::SinkExt;
    use tokio_tungstenite::tungstenite::Message;

    let (stream, _response) = tokio_tungstenite::connect_async(&config.websocket_address)
        .await
        .map_err(|e| format!("Failed to connect to Bitfinex: {}", e))?;

    let (mut write, read) = stream.split();

    // Subscribe to all configured symbols
    for symbol in &config.track_symbols {
        let subscribe_msg = serde_json::json!({
            "event": "subscribe",
            "channel": "ticker",
            "symbol": symbol
        });
        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .map_err(|e| format!("Failed to send subscription: {}", e))?;
    }

    Ok(read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<BitfinexMessage>(&text) {
                    Ok(BitfinexMessage::Ticker(data)) => {
                        BitfinexTicker::try_from(data).ok().map(Ok)
                    }
                    Ok(BitfinexMessage::Event(_)) => None, // Skip subscription confirmations
                    Err(e) => Some(Err(format!("Failed to parse Bitfinex message: {}", e))),
                }
            }
            Ok(_) => None,
            Err(e) => Some(Err(format!("WebSocket error: {}", e))),
        }
    }))
}

#[ignore]
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
