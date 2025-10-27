//! "processor" for a single asset pair (ap) orderbook using Actor pattern
//!
//! Each asset pair, like BTC/USD, gets a task spawned to process messages
//! (commands) to modify the orderbook.
//!
//! Relevant types when using this module (the interface):
//! - [`MsgIn`]
//! - [`MsgOut`]
//! - [`Error`]
//! - [`Response`]
//! - [`Envelope`]
//!

/// all this has be to be is a unique number per "user", it could be a database PK, it could be a counter, it doesn't matter!
pub type VirtualUserId = i32;

pub mod money_accounts;
pub mod order_management;
pub mod proc;
pub mod reserve_money;
pub mod test;
pub mod user_profile;
