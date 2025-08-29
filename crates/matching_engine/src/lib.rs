#![warn(missing_docs)]
//! Matching engine and service layer

pub mod asset_code;
pub mod asset_pair;
pub mod cancel_order;
pub mod decimal;
pub mod order_uuid;
pub mod orderbook;
pub mod pending_fill;
pub mod place_order;
pub mod reserve_ok;
pub mod self_trade_protection;
pub mod svc;
pub mod timeinforce;
pub mod try_fill_order;
