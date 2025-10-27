#![allow(warnings)]

pub mod v_binance;
pub mod v_bitfinex;
pub mod v_bitstamp;
pub mod v_bybit;
pub mod v_coinbase;
pub mod v_gate_io;
pub mod v_gemini;
pub mod v_kraken;
pub mod v_kucoin;
pub mod v_okx;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetFeedConfig {
    pub binance: Option<v_binance::BinanceConfig>,
    pub bitfinex: Option<v_bitfinex::BitfinexConfig>,
    pub bitstamp: Option<v_bitstamp::BitstampConfig>,
    pub bybit: Option<v_bybit::BybitConfig>,
    pub coinbase: Option<v_coinbase::CoinbaseConfig>,
    pub gate_io: Option<v_gate_io::GateIoConfig>,
    pub gemini: Option<v_gemini::GeminiConfig>,
    pub kraken: Option<v_kraken::KrakenConfig>,
    pub kucoin: Option<v_kucoin::KucoinConfig>,
    pub okx: Option<v_okx::OkxConfig>,
}

pub struct AssetFeed {
    config: AssetFeedConfig,
}

pub fn asset_feed(config: AssetFeedConfig) -> AssetFeed {
    AssetFeed { config }
}

/// Unified ticker data across all venues
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnifiedTicker {
    /// Exchange/venue name (e.g., "binance", "kraken")
    pub venue: String,
    /// Trading pair symbol (e.g., "BTC/USD", "BTCUSDT")
    pub symbol: String,
    /// Timestamp in milliseconds since Unix epoch
    pub timestamp: u64,
    /// Last traded price
    pub last_price: f64,
    /// Best bid price
    pub bid: Option<f64>,
    /// Best ask price
    pub ask: Option<f64>,
    /// 24-hour trading volume
    pub volume_24h: Option<f64>,
    /// 24-hour high price
    pub high_24h: Option<f64>,
    /// 24-hour low price
    pub low_24h: Option<f64>,
}

/// Connect to all configured venues and return a merged stream of UnifiedTicker
pub async fn connect_all<'a>(
    config: &'a AssetFeedConfig,
) -> Result<
    impl futures::Stream<Item = Result<UnifiedTicker, String>> + 'a,
    Box<dyn std::error::Error + Send + Sync>,
> {
    use futures::stream::StreamExt;

    let mut streams: Vec<
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<UnifiedTicker, String>> + Send + 'a>>,
    > = Vec::new();

    // Connect to Binance if configured
    if let Some(ref binance_config) = config.binance {
        match v_binance::connect(binance_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|event| event.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Binance: {}", e),
        }
    }

    // Connect to Bitfinex if configured
    if let Some(ref bitfinex_config) = config.bitfinex {
        match v_bitfinex::connect(bitfinex_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|ticker| ticker.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Bitfinex: {}", e),
        }
    }

    // Connect to Bitstamp if configured
    if let Some(ref bitstamp_config) = config.bitstamp {
        match v_bitstamp::connect(bitstamp_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|trade| trade.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Bitstamp: {}", e),
        }
    }

    // Connect to Bybit if configured
    if let Some(ref bybit_config) = config.bybit {
        match v_bybit::connect(bybit_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|msg| msg.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Bybit: {}", e),
        }
    }

    // Connect to Coinbase if configured
    if let Some(ref coinbase_config) = config.coinbase {
        match v_coinbase::connect(coinbase_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|ticker| ticker.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Coinbase: {}", e),
        }
    }

    // Connect to Gate.io if configured
    if let Some(ref gate_io_config) = config.gate_io {
        match v_gate_io::connect(gate_io_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|result| result.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Gate.io: {}", e),
        }
    }

    // Connect to Gemini if configured
    if let Some(ref gemini_config) = config.gemini {
        match v_gemini::connect(gemini_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|trade| trade.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Gemini: {}", e),
        }
    }

    // Connect to Kraken if configured
    if let Some(ref kraken_config) = config.kraken {
        match v_kraken::connect(kraken_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|data| data.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Kraken: {}", e),
        }
    }

    // Connect to Kucoin if configured
    if let Some(ref kucoin_config) = config.kucoin {
        match v_kucoin::connect(kucoin_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|msg| msg.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to Kucoin: {}", e),
        }
    }

    // Connect to OKX if configured
    if let Some(ref okx_config) = config.okx {
        match v_okx::connect(okx_config).await {
            Ok(stream) => {
                let unified = stream.map(|result| result.map(|data| data.into()));
                streams.push(Box::pin(unified));
            }
            Err(e) => eprintln!("Failed to connect to OKX: {}", e),
        }
    }

    if streams.is_empty() {
        return Err("No venues configured".into());
    }

    Ok(futures::stream::select_all(streams))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a sample UnifiedTicker for testing
    fn create_sample_ticker() -> UnifiedTicker {
        UnifiedTicker {
            venue: "test_venue".to_owned(),
            symbol: "BTC/USD".to_owned(),
            timestamp: 1234567890000,
            last_price: 42000.0,
            bid: Some(41999.5),
            ask: Some(42000.5),
            volume_24h: Some(1000.0),
            high_24h: Some(43000.0),
            low_24h: Some(41000.0),
        }
    }

    #[test]
    fn test_unified_ticker_creation() {
        let ticker = create_sample_ticker();

        assert_eq!(ticker.venue, "test_venue");
        assert_eq!(ticker.symbol, "BTC/USD");
        assert_eq!(ticker.timestamp, 1234567890000);
        assert_eq!(ticker.last_price, 42000.0);
        assert_eq!(ticker.bid, Some(41999.5));
        assert_eq!(ticker.ask, Some(42000.5));
        assert_eq!(ticker.volume_24h, Some(1000.0));
        assert_eq!(ticker.high_24h, Some(43000.0));
        assert_eq!(ticker.low_24h, Some(41000.0));
    }

    #[tokio::test]
    async fn test_connect_all_empty_config() {
        let config = AssetFeedConfig {
            binance: None,
            bitfinex: None,
            bitstamp: None,
            bybit: None,
            coinbase: None,
            gate_io: None,
            gemini: None,
            kraken: None,
            kucoin: None,
            okx: None,
        };

        let result = connect_all(&config).await;

        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error.to_string(), "No venues configured");
    }

    #[test]
    fn test_asset_feed_config_creation() {
        let config = AssetFeedConfig {
            binance: Some(v_binance::BinanceConfig {
                websocket_address: "wss://test.com".to_owned(),
                track_symbols: vec!["btcusdt".to_owned()],
            }),
            bitfinex: None,
            bitstamp: None,
            bybit: None,
            coinbase: None,
            gate_io: None,
            gemini: None,
            kraken: None,
            kucoin: None,
            okx: None,
        };

        assert!(config.binance.is_some());
        assert!(config.bitfinex.is_none());
        assert_eq!(config.binance.as_ref().unwrap().track_symbols.len(), 1);
    }

    #[test]
    fn test_asset_feed_creation() {
        let config = AssetFeedConfig {
            binance: None,
            bitfinex: None,
            bitstamp: None,
            bybit: None,
            coinbase: None,
            gate_io: None,
            gemini: None,
            kraken: None,
            kucoin: None,
            okx: None,
        };

        let feed = asset_feed(config);
        // Just verify it compiles and creates successfully
        assert!(std::mem::size_of_val(&feed) > 0);
    }

    /// Integration test that connects to real venues and pulls actual market data
    /// Run with: cargo test test_real_venue_connections_integration -- --ignored --nocapture
    #[ignore]
    #[tokio::test]
    async fn test_real_venue_connections_integration() {
        use futures::StreamExt;

        println!("\n=== Starting Real Venue Integration Test ===\n");

        // Configure real venues with their actual WebSocket addresses
        let config = AssetFeedConfig {
            binance: Some(v_binance::BinanceConfig {
                websocket_address: v_binance::DEFAULT_WEBSOCKET_ADDRESS.to_string(),
                track_symbols: vec!["btcusdt".to_owned()],
            }),
            bitfinex: None,
            bitstamp: None,
            bybit: None,
            coinbase: None,
            gate_io: None,
            gemini: None,
            kraken: Some(v_kraken::KrakenConfig {
                websocket_address: v_kraken::DEFAULT_WEBSOCKET_ADDRESS.to_string(),
                track_symbols: vec!["BTC/USD".to_owned()],
            }),
            kucoin: None,
            okx: None,
        };

        // Connect to all configured venues
        let mut stream = connect_all(&config)
            .await
            .expect("Failed to connect to venues");

        println!("Successfully connected to venues, waiting for data...\n");

        let mut tickers_received = 0;
        let max_tickers = 10;
        let mut venues_seen = std::collections::HashSet::new();

        // Collect first 10 ticker events
        while tickers_received < max_tickers {
            match tokio::time::timeout(std::time::Duration::from_secs(30), stream.next()).await {
                Ok(Some(Ok(ticker))) => {
                    tickers_received += 1;
                    venues_seen.insert(ticker.venue.clone());

                    println!("Ticker #{}: {:?}", tickers_received, ticker);
                    println!(
                        "  Venue: {}, Symbol: {}, Price: ${:.2}",
                        ticker.venue, ticker.symbol, ticker.last_price
                    );
                    if let (Some(bid), Some(ask)) = (ticker.bid, ticker.ask) {
                        println!("  Bid: ${:.2}, Ask: ${:.2}", bid, ask);
                    }
                    println!();

                    // Validate ticker data
                    assert!(!ticker.venue.is_empty(), "Venue name should not be empty");
                    assert!(!ticker.symbol.is_empty(), "Symbol should not be empty");
                    assert!(ticker.last_price > 0.0, "Last price should be positive");
                    assert!(ticker.timestamp > 0, "Timestamp should be set");
                }
                Ok(Some(Err(e))) => {
                    println!("Stream error: {}", e);
                }
                Ok(None) => {
                    println!("Stream ended unexpectedly");
                    break;
                }
                Err(_) => {
                    println!("Timeout waiting for ticker data");
                    break;
                }
            }
        }

        println!("\n=== Test Summary ===");
        println!("Total tickers received: {}", tickers_received);
        println!("Venues that sent data: {:?}", venues_seen);

        // Verify we got data from at least one venue
        assert!(
            tickers_received > 0,
            "Should have received at least one ticker"
        );
        assert!(
            !venues_seen.is_empty(),
            "Should have received data from at least one venue"
        );

        println!("\n=== Integration Test PASSED ===\n");
    }
}
