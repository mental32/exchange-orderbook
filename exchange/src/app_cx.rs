//! "app_cx" is a horrible name, but I can't think of anything better. Basically
//! it's a struct that holds all the data/refs that the different tasks need
//! access to. It's a bit like a global variable. app_cx is short for "application context"
//!
//! it is also a facade for the different components of the exchange. For
//! example, instead of calling `te_tx.send(TradingEngineCmd::PlaceOrder { .. })`
//! you would call `app.place_order(..)`.
//!
use std::convert::Infallible;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::oneshot;
use uuid::Uuid;

use crate::bitcoin::BitcoinRpcClient;
use crate::trading::{
    OrderSide, OrderUuid, PlaceOrder, TradeCmd, TradingEngineCmd, TradingEngineError,
    TradingEngineTx,
};
use crate::web::TradeAddOrder;

use super::*;

#[derive(Debug, Clone)]
pub struct AppCx {
    /// a mpsc sender to the trading engine supervisor.
    te_tx: TradingEngineTx,
    /// a client for the bitcoin core rpc.
    bitcoind_rpc: BitcoinRpcClient,
    /// a pool of connections to the database.
    pub(crate) db_pool: sqlx::PgPool,
    /// Read-only data or data that has interior mutability.
    inner_ro: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    te_suspended: AtomicBool,
}

pub enum TradingEngineState {
    Suspended,
    Running,
}

#[derive(Debug, Error)]
pub enum PlaceOrderError {
    #[error("trading engine unresponsive")]
    TradingEngineUnresponsive,
}

#[must_use]
pub struct Response<T, E = TradingEngineError>(pub oneshot::Receiver<Result<T, E>>);

impl<T, E> Response<T, E> {
    pub async fn wait(self) -> Option<Result<T, E>> {
        self.0.await.ok()
    }
}

impl AppCx {
    pub fn new(te_tx: TradingEngineTx, btc_rpc: BitcoinRpcClient, db_pool: sqlx::PgPool) -> Self {
        Self {
            te_tx,
            bitcoind_rpc: btc_rpc,
            db_pool,
            inner_ro: Arc::new(Inner {
                te_suspended: AtomicBool::new(false),
            }),
        }
    }

    pub fn trading_engine_state(&self) -> TradingEngineState {
        if self.inner_ro.te_suspended.load(Ordering::Relaxed) {
            TradingEngineState::Suspended
        } else {
            TradingEngineState::Running
        }
    }

    pub fn suspend_trading_engine(&self) {
        self.inner_ro.te_suspended.store(true, Ordering::SeqCst);
    }

    pub fn resume_trading_engine(&self) {
        self.inner_ro.te_suspended.store(false, Ordering::SeqCst);
    }

    pub async fn place_order(
        &self,
        asset: Asset,
        user_uuid: uuid::Uuid,
        trade_add_order: TradeAddOrder,
    ) -> Result<Response<OrderUuid>, PlaceOrderError> {
        if self.inner_ro.te_suspended.load(Ordering::Relaxed) {
            return Err(PlaceOrderError::TradingEngineUnresponsive);
        }

        let TradeAddOrder {
            side,
            order_type,
            stp,
            quantity,
            price,
            time_in_force,
        } = trade_add_order;

        let (place_order_tx, wait_response) = oneshot::channel();
        let place_order = PlaceOrder::new(
            asset,
            user_uuid,
            price,
            quantity,
            order_type,
            stp,
            time_in_force,
            side,
        );

        let cmd = TradeCmd::PlaceOrder((place_order, place_order_tx));

        self.te_tx
            .send(TradingEngineCmd::Trade(cmd))
            .await
            .map_err(|_| PlaceOrderError::TradingEngineUnresponsive)?;

        Ok(Response(wait_response))
    }
}
