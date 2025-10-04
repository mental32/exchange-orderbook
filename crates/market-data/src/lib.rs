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
    // pub binance: Option<binance::BinanceConfig>,
    // pub bitfinex: Option<bitfinex::BitfinexConfig>,
    // pub bitstamp: Option<bitstamp::BitstampConfig>,
    // pub bybit: Option<bybit::BybitConfig>,
    // pub coinbase: Option<coinbase::CoinbaseConfig>,
    // pub gate_io: Option<gate_io::GateIoConfig>,
    // pub gemini: Option<gemini::GeminiConfig>,
    pub kraken: Option<v_kraken::KrakenConfig>,
    // pub kucoin: Option<kucoin::KucoinConfig>,
    // pub okx: Option<okx::OkxConfig>,
}

pub struct AssetFeed {
    config: AssetFeedConfig,
}

pub fn asset_feed(config: AssetFeedConfig) -> AssetFeed {
    AssetFeed { config }
}
