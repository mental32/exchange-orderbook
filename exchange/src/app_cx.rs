//! "app_cx" is a horrible name, but I can't think of anything better. Basically
//! it's a struct that holds all the data/refs that the different tasks need
//! access to. It's a bit like a global variable. app_cx is short for "application context"
//!
//! it is also a facade for the different components of the exchange. For
//! example, instead of calling `te_tx.send(TradingEngineCmd::PlaceOrder { .. })`
//! you would call `app.place_order(..)`.
//!
use std::num::NonZeroU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use atomic::Atomic;
use futures::TryFutureExt;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::bitcoin::BitcoinRpcClient;
use crate::trading::{
    OrderSide, OrderUuid, PlaceOrder, TradeCmd, TradingEngineCmd, TradingEngineError,
    TradingEngineTx,
};
use crate::web::TradeAddOrder;

use super::*;

pub struct DeferGuard<F: FnMut()> {
    f: F,
    active: bool,
}

impl<F: FnMut()> Drop for DeferGuard<F> {
    fn drop(&mut self) {
        if self.active {
            (self.f)();
        }
    }
}

impl<F: FnMut()> DeferGuard<F> {
    pub fn cancel(mut self) {
        self.active = false;
    }
}

#[must_use]
pub fn defer<F: FnMut()>(f: F) -> DeferGuard<F> {
    DeferGuard { f, active: true }
}

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
    te_state: Atomic<TradingEngineState>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum TradingEngineState {
    #[default]
    Suspended = 0,
    Running,
    ReduceOnly,
}

unsafe impl bytemuck::NoUninit for TradingEngineState {}

#[derive(Debug, Clone)]
pub struct ReserveOk {
    pub row_id: u32,
    pub previous_balance: NonZeroU64,
    pub new_balance: Option<NonZeroU64>,
}

impl ReserveOk {
    pub fn defer_revert(
        self,
        handle: tokio::runtime::Handle,
        db: sqlx::PgPool,
    ) -> DeferGuard<impl FnMut()> {
        defer(move || {
            let this = self.clone();
            let db = db.clone();

            handle.spawn(async move {
                let fut = this.revert(&db);

                if let Err(err) = fut.await {
                    tracing::warn!(?err, "failed to revert reserved funds");
                }
            });
        })
    }

    pub fn revert(
        self,
        db: &sqlx::PgPool,
    ) -> impl std::future::Future<Output = Result<i32, sqlx::Error>> + '_ {
        sqlx::query!(
            r#"
            -- First, fetch the required details from the original row
            WITH original_tx AS (
            SELECT credit_account_id, debit_account_id, currency, amount
                FROM account_tx_journal
                WHERE id = $1
            )
            -- Then, insert the inverse transaction
            INSERT INTO account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type)
            SELECT debit_account_id, credit_account_id, currency, amount, 'revert reserve asset'
            FROM original_tx
            RETURNING id
            "#,
            self.row_id as i32
        )
        .fetch_one(db)
        .map_ok(|rec| rec.id)
    }
}

#[derive(Debug, Error)]
pub enum ReserveError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

impl From<ReserveError> for PlaceOrderError {
    fn from(value: ReserveError) -> Self {
        match value {
            ReserveError::InsufficientFunds => PlaceOrderError::InsufficientFunds,
            ReserveError::Database(_) => todo!("internal error"),
        }
    }
}

#[derive(Debug, Error)]
pub enum PlaceOrderError {
    #[error("trading engine unresponsive")]
    TradingEngineUnresponsive,
    #[error("insufficient funds")]
    InsufficientFunds,
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
                te_state: Atomic::new(TradingEngineState::Running),
            }),
        }
    }

    /// get the state of the trading engine
    pub fn trading_engine_state(&self) -> TradingEngineState {
        self.inner_ro.te_state.load(Ordering::Relaxed)
    }

    /// set the state of the trading engine
    pub fn set_trading_engine_state(&self, state: TradingEngineState) {
        self.inner_ro.te_state.store(state, Ordering::SeqCst)
    }

    async fn calculate_balance(
        &self,
        user_uuid: Uuid,
        currency: &str,
    ) -> Result<Option<NonZeroU64>, sqlx::Error> {
        let rec = sqlx::query!(
            r#"
            WITH account_id AS (
                SELECT id FROM accounts 
                WHERE source_type = 'user' AND source_id = $1 AND currency = $2
            )
            SELECT (
                (SELECT COALESCE(SUM(amount), 0) FROM account_tx_journal WHERE credit_account_id = (SELECT id FROM account_id))::BIGINT -
                (SELECT COALESCE(SUM(amount), 0) FROM account_tx_journal WHERE debit_account_id = (SELECT id FROM account_id))::BIGINT
            ) AS balance
            "#,
            user_uuid.to_string(),
            currency
        ).fetch_one(&self.db_pool).await?.balance;
        tracing::trace!(?rec, %user_uuid, ?currency, "balance");
        Ok(NonZeroU64::new(rec.unwrap_or_default() as u64))
    }

    async fn reserve_by_asset(
        &self,
        user_uuid: Uuid,
        quantity: std::num::NonZeroU32,
        currency: &str,
    ) -> Result<ReserveOk, ReserveError> {
        let balance = self.calculate_balance(user_uuid, currency).await?;

        let balance = match balance {
            Some(i) if i.get() >= quantity.get() as u64 => i,
            _ => return Err(ReserveError::InsufficientFunds),
        };

        // create a new account_tx_journal record to debit the user's account for the reserved amount.
        let rec = sqlx::query!(
            r#"
            INSERT INTO account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type) VALUES (
                (SELECT id FROM accounts WHERE source_type = 'fiat' AND source_id = 'exchange' AND currency = $3),
                (SELECT id FROM accounts WHERE source_type = 'user' AND source_id = $2),
                $3,
                $1,
                'reserve asset'
            ) RETURNING id
            "#,
            quantity.get() as i64,
            user_uuid.to_string(),
            currency,
        ).fetch_one(&self.db_pool).await?;

        tracing::trace!(id = ?rec.id, %user_uuid, "reserved USD fiat from user account");

        let new_balance = self.calculate_balance(user_uuid, currency).await?;
        if let Some(nb) = new_balance {
            assert!(nb.get() < balance.get());
        }

        Ok(ReserveOk {
            row_id: rec.id as u32,
            previous_balance: balance,
            new_balance,
        })
    }

    pub async fn place_order(
        &self,
        asset: Asset,
        user_uuid: uuid::Uuid,
        trade_add_order: TradeAddOrder,
    ) -> Result<(Response<OrderUuid>, ReserveOk), PlaceOrderError> {
        if !matches!(self.trading_engine_state(), TradingEngineState::Running) {
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

        let reserve = match side {
            OrderSide::Buy => self.reserve_by_asset(user_uuid, quantity, "USD").await?,
            OrderSide::Sell => {
                self.reserve_by_asset(
                    user_uuid,
                    quantity,
                    match asset {
                        Asset::Bitcoin => "BTC",
                        Asset::Ether => "ETH",
                    },
                )
                .await?
            }
        };

        tracing::trace!(?reserve.previous_balance, ?reserve.new_balance, "marked funds as reserved");

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

        match self.te_tx.send(TradingEngineCmd::Trade(cmd)).await {
            Ok(()) => Ok((Response(wait_response), reserve)),
            Err(err) => {
                tracing::warn!(?err, "failed to send place order command to trading engine");
                if let Err(err) = reserve.revert(&self.db_pool).await {
                    tracing::error!(?err, "failed to revert reserve");
                }
                Err(PlaceOrderError::TradingEngineUnresponsive)
            }
        }
    }
}
