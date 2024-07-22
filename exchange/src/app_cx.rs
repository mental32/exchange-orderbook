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

use argon2::password_hash::PasswordHashString;
use argon2::PasswordHasher;
use asset::{internal_asset_list, AssetKey};
use atomic::Atomic;
use futures::TryFutureExt;
use password::Password;
use sqlx::PgPool;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::bitcoin::BitcoinRpcClient;
use crate::trading::{
    CancelOrder, OrderSide, OrderUuid, PlaceOrder, PlaceOrderResult, TradeCmd, TradingEngineCmd,
    TradingEngineError, TradingEngineTx,
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
    db: sqlx::PgPool,
    /// Read-only data or data that has interior mutability.
    inner_ro: Arc<Inner>,
    pub assets: &'static [(AssetKey, Asset)],
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

#[derive(Debug, Error)]
pub enum CancelOrderError {
    #[error("trading engine unresponsive")]
    TradingEngineUnresponsive,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateUserError {
    #[error("password hash error")]
    PasswordHashError,
    #[error("email has already been used")]
    EmailUniqueViolation(sqlx::Error),
    #[error("sqlx error")]
    GenericSqlxError(#[from] sqlx::Error),
}

#[must_use]
pub struct Response<T, E = TradingEngineError>(pub oneshot::Receiver<Result<T, E>>);

impl<T, E> Response<T, E> {
    pub async fn wait(self) -> Option<Result<T, E>> {
        self.0.await.ok()
    }
}

impl AppCx {
    pub fn new(te_tx: TradingEngineTx, btc_rpc: BitcoinRpcClient, db: sqlx::PgPool) -> Self {
        Self {
            te_tx,
            bitcoind_rpc: btc_rpc,
            db,
            inner_ro: Arc::new(Inner {
                te_state: Atomic::new(TradingEngineState::Running),
            }),
            assets: internal_asset_list(),
        }
    }

    pub fn db(&self) -> PgPool {
        self.db.clone()
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
            SELECT calculate_balance($1, $2);"#,
            user_uuid.to_string(),
            currency
        )
        .fetch_one(&self.db)
        .await?
        .calculate_balance;
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
        ).fetch_one(&self.db).await?;

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
    ) -> Result<(Response<PlaceOrderResult>, ReserveOk), PlaceOrderError> {
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
                if let Err(err) = reserve.revert(&self.db).await {
                    tracing::error!(?err, "failed to revert reserve");
                }
                Err(PlaceOrderError::TradingEngineUnresponsive)
            }
        }
    }

    pub async fn cancel_order(
        &self,
        user_uuid: Uuid,
        order_uuid: Uuid,
    ) -> Result<Response<()>, CancelOrderError> {
        // Running and ReduceOnly are the only states where we can cancel orders.
        if matches!(self.trading_engine_state(), TradingEngineState::Suspended) {
            return Err(CancelOrderError::TradingEngineUnresponsive);
        }

        let (cancel_order_tx, wait_response) = oneshot::channel();
        let cancel_order = CancelOrder::new(user_uuid, OrderUuid(order_uuid));

        let cmd = TradeCmd::CancelOrder((cancel_order, cancel_order_tx));

        match self.te_tx.send(TradingEngineCmd::Trade(cmd)).await {
            Ok(()) => Ok(Response(wait_response)),
            Err(err) => {
                tracing::warn!(
                    ?err,
                    "failed to send cancel order command to trading engine"
                );
                Err(CancelOrderError::TradingEngineUnresponsive)
            }
        }
    }

    pub async fn create_user(
        &self,
        name: &str,
        email: &str,
        password_hash: PasswordHashString,
    ) -> Result<Uuid, CreateUserError> {
        // duplicate emails should raise a unique violation
        if let Err(err) = sqlx::query!(
            r#"
            INSERT INTO users (name, email, password_hash)
            VALUES (
                    $1,
                    $2,
                    $3
                );
            "#,
            name,
            email,
            password_hash.as_bytes(),
        )
        .execute(&self.db())
        .await
        {
            return Err(match err {
                sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
                    CreateUserError::EmailUniqueViolation(err)
                }
                _ => CreateUserError::GenericSqlxError(err),
            });
        }

        let rec = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
            .fetch_one(&self.db())
            .await?;

        Ok(rec.id)
    }
}

#[cfg(test)]
mod test {
    use spawn_trading_engine::spawn_trading_engine;

    use super::*;

    async fn make_app_cx_fixture(db: sqlx::PgPool) -> AppCx {
        let config = Config::load_from_toml("");
        let (te_tx, te_handle) = spawn_trading_engine(&config, db.clone())
            .init_from_db(db.clone())
            .await
            .unwrap();
        AppCx::new(te_tx, BitcoinRpcClient::new_mock(), db)
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_duplicate_user_email(db: sqlx::PgPool) {
        let app_cx = make_app_cx_fixture(db.clone()).await;
        let password_hash = Password("letmein".into());

        let user_uuid = app_cx
            .create_user(
                "foo",
                "foo@example.com",
                password_hash.argon2_hash_password().unwrap(),
            )
            .await
            .unwrap();

        if let Err(err) = app_cx
            .create_user(
                "foo",
                "foo@example.com",
                password_hash.argon2_hash_password().unwrap(),
            )
            .await
        {
            assert!(matches!(err, CreateUserError::EmailUniqueViolation(err)));
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_calculate_balances(db: sqlx::PgPool) {
        let app_cx = make_app_cx_fixture(db.clone()).await;

        let password_hash = Password("letmein".into()).argon2_hash_password().unwrap();
        let user_uuid = app_cx
            .create_user("foo", "foo@example.com", password_hash)
            .await
            .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO accounts (source_type, source_id, currency)
            VALUES ('user', $1, 'BTC');
            "#,
            user_uuid.to_string()
        )
        .execute(&db)
        .await
        .unwrap();

        // Generate a random number of transactions
        let num_transactions = rand::random::<u8>() as usize; // generate a random number of transactions
        let mut total_credits: i64 = 0;

        for _ in 0..num_transactions {
            let amount = (rand::random::<u16>() + 1) as i64; // ensure non-zero
            total_credits += amount;

            sqlx::query!(
                r#"
                INSERT INTO account_tx_journal (credit_account_id, debit_account_id, currency, amount, transaction_type)
                VALUES ((SELECT id FROM accounts WHERE source_id = $1 AND currency = 'BTC'), 1, 'BTC', $2, 'random deposit');
                "#,
                user_uuid.to_string(),
                amount
            )
            .execute(&db)
            .await
            .unwrap();
        }

        let balance = app_cx.calculate_balance(user_uuid, "BTC").await.unwrap();

        assert_eq!(balance, NonZeroU64::new(total_credits as u64), "Expected balance does not match calculated balance: user={user_uuid} balance={balance:?} expected={total_credits:?}");
    }
}
