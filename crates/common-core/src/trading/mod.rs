//! Trading module for the exchange, contains the orderbook and order matching logic.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

// /// Error that can occur when interacting with the trading engine.
// #[derive(Debug, Error)]
// pub enum TradingEngineError {
//     /// the trading engine is suspended
//     #[error("the trading engine is suspended")]
//     Suspended,
//     /// unserializable input to trading engine
//     #[error("unserializable input to trading engine")]
//     UnserializableInput,
//     /// order not found
//     #[error("order not found for user {0:?} and order uuid {1:?}")]
//     OrderNotFound(uuid::Uuid, OrderUuid),
//     /// database error
//     #[error("database error")]
//     Database(#[from] sqlx::Error),
//     /// error that can occur when executing a pending fill operation.
//     #[error("place order error")]
//     PlaceOrder(#[from] PlaceOrderError),
// }

#[derive(Debug, thiserror::Error)]
#[error("order not found for user {0:?} and order uuid {1:?}")]
pub struct OrderNotFound(uuid::Uuid, order_uuid::OrderUuid);

pub mod asset;
pub mod cancel_order;
pub mod order_uuid;
pub mod orderbook;
pub mod pending_fill;
pub mod place_order;
pub mod self_trade_protection;
pub mod timeinforce;
pub mod try_fill_order;
