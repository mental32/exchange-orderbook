//! "app_cx" is a horrible name, but I can't think of anything better. Basically
//! it's a struct that holds all the data/refs that the different tasks need
//! access to. It's a bit like a global variable. app_cx is short for "application context"
//!
//! it is also a facade for the different components of the exchange. For
//! example, instead of calling `te_tx.send(TradingEngineCmd::PlaceOrder { .. })`
//! you would call `app.place_order(..)`.
//!
use std::sync::Arc;

use crate::trading::TradingEngineTx;

use super::*;

#[derive(Debug, Clone)]
pub struct AppCx {
    /// Read-only data or data that has interior mutability.
    inner_ro: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    te_tx: TradingEngineTx,
}

impl AppCx {
    pub fn new(te_tx: TradingEngineTx) -> Self {
        Self {
            inner_ro: Arc::new(Inner { te_tx }),
        }
    }

    pub async fn place_order(
        &self,
        asset: Asset,
        order_type: crate::trading::OrderType,
        stp: crate::trading::SelfTradeProtection,
        side: crate::trading::OrderSide,
    ) {
        todo!()
    }
}
