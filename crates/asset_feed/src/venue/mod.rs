//! Asset feed connects to various exchanges and calculates price changes for assets.
//!

// pub mod binance;
// pub mod bitfinex;
// pub mod bitstamp;
// pub mod bybit;
// pub mod coinbase;
// pub mod gate_io;
// pub mod gemini;
// pub mod kraken;
// pub mod kucoin;
// pub mod okx;

use std::future::Future;

use serde::{Deserialize, Serialize};
use tracing::Instrument;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetFeedConfig {
    // pub binance: Option<binance::BinanceConfig>,
    // pub bitfinex: Option<bitfinex::BitfinexConfig>,
    // pub bitstamp: Option<bitstamp::BitstampConfig>,
    // pub bybit: Option<bybit::BybitConfig>,
    // pub coinbase: Option<coinbase::CoinbaseConfig>,
    // pub gate_io: Option<gate_io::GateIoConfig>,
    // pub gemini: Option<gemini::GeminiConfig>,
    // pub kraken: Option<kraken::KrakenConfig>,
    // pub kucoin: Option<kucoin::KucoinConfig>,
    // pub okx: Option<okx::OkxConfig>,
}

pub struct AssetFeed {
    config: AssetFeedConfig,
}

pub fn asset_feed(config: AssetFeedConfig) -> AssetFeed {
    AssetFeed { config }
}
