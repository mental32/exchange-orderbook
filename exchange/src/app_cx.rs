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

#[derive(Debug, Clone)]
pub struct ReserveFiat {
    pub previous_balance: NonZeroU64,
    pub new_balance: Option<NonZeroU64>,
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

    async fn fiat_balance(&self, user_uuid: Uuid) -> Result<Option<NonZeroU64>, sqlx::Error> {
        let rec = sqlx::query!(
            r#"
            WITH account_id AS (
                SELECT id FROM accounts 
                WHERE source_type = 'user' AND source_id = $1
            )
            SELECT (
                (SELECT COALESCE(SUM(amount), 0) FROM account_tx_journal WHERE credit_account_id = (SELECT id FROM account_id))::BIGINT -
                (SELECT COALESCE(SUM(amount), 0) FROM account_tx_journal WHERE debit_account_id = (SELECT id FROM account_id))::BIGINT
            ) AS balance
            "#,
            user_uuid.to_string()
        ).fetch_one(&self.db_pool).await?.balance;
        tracing::trace!(?rec, %user_uuid, "fiat balance");
        Ok(NonZeroU64::new(rec.unwrap_or_default() as u64))
    }

    async fn reserve_fiat(
        &self,
        user_uuid: Uuid,
        quantity: std::num::NonZeroU32,
    ) -> Result<ReserveFiat, ReserveError> {
        let balance = self.fiat_balance(user_uuid).await?;

        let balance = match balance {
            Some(i) if i.get() >= quantity.get() as u64 => i,
            _ => return Err(ReserveError::InsufficientFunds),
        };

        // create a new account_tx_journal record to debit the user's account for the reserved amount.
        let _res = sqlx::query!(
            r#"
            INSERT INTO account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type) VALUES (
                (SELECT id FROM accounts WHERE source_type = 'fiat' AND source_id = 'exchange' AND currency = 'USD'),
                (SELECT id FROM accounts WHERE source_type = 'user' AND source_id = $2),
                'USD',
                $1,
                'reserve fiat'
            );            
            "#,
            quantity.get() as i64,
            user_uuid.to_string()
        ).execute(&self.db_pool).await?;

        tracing::trace!(?_res, %user_uuid, "reserved USD fiat from user account");

        let new_balance = self.fiat_balance(user_uuid).await?;
        if let Some(nb) = new_balance {
            assert!(nb.get() < balance.get());
        }

        Ok(ReserveFiat {
            previous_balance: balance,
            new_balance,
        })
    }

    async fn reserve_crypto(
        &self,
        user_uuid: Uuid,
        quantity: std::num::NonZeroU32,
        asset: Asset,
    ) -> Result<(), ReserveError> {
        todo!()
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

        match side {
            OrderSide::Buy => {
                self.reserve_fiat(user_uuid, quantity).await?;
            }
            OrderSide::Sell => self.reserve_crypto(user_uuid, quantity, asset).await?,
        };

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
