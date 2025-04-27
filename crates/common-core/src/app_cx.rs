//! "app_cx" is short for "application context" used to access and share access to other core parts.
//!
//! "app_cx" is a horrible name, but I can't think of anything better. Basically
//! it's a struct that holds all the data/refs that the different tasks need
//! access to. It's a bit like a global variable. app_cx is short for "application context"
//!
//! it is a facade for the different components of the exchange. For
//! example, instead of calling `te_tx.send(TradingEngineCmd::PlaceOrder { .. })`
//! you would call `app.place_order(..)`.
//!
use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU64;
use std::path::Path;
use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use argon2::password_hash::PasswordHashString;
use argon2::{Argon2, PasswordHasher, PasswordVerifier as _};
use atomic::Atomic;
use email_address::EmailAddress;
use futures::TryFutureExt;
use mime_guess::MimeGuess;
use minijinja_autoreload::AutoReloader;
use serde::Serialize;
use sqlx::{Executor as _, PgPool};
use thiserror::Error;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::asset::{AssetKey, internal_asset_list};
use crate::bitcoin::BitcoinRpcClient;
use crate::password::Password;
use crate::trading::{
    CancelOrder, OrderSide, OrderUuid, PlaceOrder, PlaceOrderResult, TeResponse as Response,
    TradeCmd, TradingEngineCmd, TradingEngineError, TradingEngineTx,
};
use crate::web::TradeAddOrder;
use crate::{Asset, Configuration};

mod defer_guard;
pub use defer_guard::{DeferGuard, defer};

mod reserve_ok;
pub use reserve_ok::ReserveOk;

struct Inner {
    te_state: Atomic<TradingEngineState>,
    // jinja: crate::jinja::Jinja,
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
pub enum VerifyLoginDetailsError {
    #[error("failed to authorize details")]
    Unauthorized,
    #[error("{0}")]
    Other(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
pub enum CancelOrderError {
    #[error("trading engine unresponsive")]
    TradingEngineUnresponsive,
}

#[derive(Debug, Error)]
pub enum CreateUserError {
    #[error("password hash error")]
    PasswordHashError,
    #[error("email has already been used")]
    EmailUniqueViolation(sqlx::Error),
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
pub enum UserDetailsError {
    #[error("sqlx: (0]")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Serialize)]
pub struct UserWalletAddr {
    text: String,
    currency: String,
    kind: String,
}

#[derive(Debug, Serialize)]
pub struct UserAccountDetails {
    amount: String,
    deposit_address: Option<String>,
    withrawal_addresses: Vec<UserWalletAddr>,
}

#[derive(Debug, Serialize)]
pub struct UserPortfolio {
    value: usize,
}

#[derive(Debug, Serialize)]
pub struct UserDetails {
    id: uuid::Uuid,
    name: String,
    role: String,
    accounts: HashMap<String, UserAccountDetails>,
    portfolio: UserPortfolio,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
enum TradingEngineState {
    #[default]
    Suspended = 0,
    Running,
    ReduceOnly,
}

unsafe impl bytemuck::NoUninit for TradingEngineState {}

#[derive(Debug)]
pub struct UserAccount {}

#[derive(Debug, Clone)]
pub struct AppCx {
    /// a mpsc sender to the trading engine supervisor.
    te_tx: TradingEngineTx,
    /// a client for the bitcoin core rpc.
    pub(crate) bitcoind_rpc: BitcoinRpcClient,
    /// a pool of connections to the database.
    db: sqlx::PgPool,
    /// Read-only data or data that has interior mutability.
    inner_ro: Arc<Inner>,
    /// The service configuration
    config: Configuration,
    /// The list of active assets
    pub(crate) assets: &'static [(AssetKey, Asset)],
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("te_state", &self.te_state)
            .field("jinja", &"")
            .finish()
    }
}

impl AppCx {
    pub fn new(
        te_tx: TradingEngineTx,
        btc_rpc: BitcoinRpcClient,
        db: sqlx::PgPool,
        config: Configuration,
    ) -> Self {
        Self {
            te_tx,
            bitcoind_rpc: btc_rpc,
            db,
            inner_ro: Arc::new(Inner {
                te_state: Atomic::new(TradingEngineState::Running),
            }),
            assets: internal_asset_list(),
            config,
        }
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn db(&self) -> PgPool {
        self.db.clone()
    }

    pub fn trading_engine_state(&self) -> TradingEngineState {
        self.inner_ro.te_state.load(Ordering::Relaxed)
    }

    pub fn set_trading_engine_state(&self, state: TradingEngineState) {
        self.inner_ro.te_state.store(state, Ordering::SeqCst)
    }
}

impl AppCx {
    pub async fn list_withdrawal_addrs(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        Ok(sqlx::query!(
            "SELECT address_text, currency
                FROM user_addresses
                WHERE user_id = $1
                AND kind = 'withdrawal';",
            user_id
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|rec| (rec.address_text, rec.currency))
        .collect())
    }

    pub async fn list_deposit_addrs(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        Ok(sqlx::query!(
            "SELECT address_text, currency
                FROM user_addresses
                WHERE user_id = $1
                AND kind = 'deposit';",
            user_id
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|rec| (rec.address_text, rec.currency))
        .collect())
    }

    pub async fn calculate_balance_from_accounting(
        &self,
        user_id: Uuid,
        currency: &str,
    ) -> Result<Option<NonZeroU64>, sqlx::Error> {
        let rec = sqlx::query!(
            r#"
            SELECT calculate_balance($1, $2);"#,
            user_id.to_string(),
            currency
        )
        .fetch_one(&self.db)
        .await?
        .calculate_balance;
        tracing::trace!(?rec, %user_id, ?currency, "balance");
        Ok(NonZeroU64::new(rec.unwrap_or_default() as u64))
    }

    pub async fn update_user_accounts(&self, user_id: Uuid) {
        async fn check_bitcoind(mut cx: AppCx, user_id: Uuid) -> Result<(), sqlx::Error> {
            use crate::bitcoin::proto::ListTransactionsRequest;

            let _db = cx.db();
            let mut db = _db.begin().await?;

            let btc_account_rec = sqlx::query!(
                r#"SELECT id FROM accounts WHERE source_type = 'crypto' AND source_id = 'bitcoin';"#
            )
            .fetch_one(&mut *db)
            .await?;

            let user_account_rec = sqlx::query!(
                "SELECT * FROM accounts WHERE source_id = $1 AND currency = 'BTC' AND source_type = 'user';",
                user_id.to_string()
            )
            .fetch_one(&mut *db)
            .await?;

            let mut tx_journal = sqlx::query!("SELECT * FROM account_tx_journal WHERE credit_account_id = $1 AND debit_account_id = $2 AND currency = 'BTC' AND transaction_type = 'CHAIN.DEPOSIT';", user_account_rec.id, btc_account_rec.id)
                .fetch_all(&mut *db)
                .await?
                .into_iter()
                .map(|rec| (rec.txid.clone(), rec))
                .collect::<HashMap<_, _>>();

            let txs = cx
                .bitcoind_rpc
                .list_transactions(ListTransactionsRequest {
                    label: Some(user_id.to_string()),
                    count: None,
                    skip: None,
                    include_watch_only: None,
                })
                .await
                .unwrap()
                .into_inner();

            for tx in txs.transactions {
                if tx_journal.contains_key(&tx.txid) {
                    continue;
                }

                let res = sqlx::query!(
                    r#"INSERT INTO account_tx_journal (
                        credit_account_id,
                        debit_account_id,
                        currency,
                        amount,
                        transaction_type,
                        txid
                    ) VALUES ($1, $2, 'BTC', $3, 'CHAIN.DEPOSIT', $4)"#,
                    user_account_rec.id,
                    btc_account_rec.id,
                    tx.amount as i64,
                    tx.txid
                )
                .execute(&mut *db)
                .await?;
            }

            db.commit().await?;

            Ok(())
        }

        let check_bitcoind_fut = check_bitcoind(self.clone(), user_id.clone());
        let (res,) = tokio::join!(check_bitcoind_fut);
    }

    pub async fn user_balance(&self, user_id: Uuid) -> Result<HashMap<String, i64>, sqlx::Error> {
        let mut db = self.db.begin().await?;
        let mut details = HashMap::new();

        let vec = sqlx::query!(
            "SELECT DISTINCT currency FROM accounts WHERE source_id = $1;",
            user_id.to_string()
        )
        .fetch_all(&mut *db)
        .await?;

        for rec in vec {
            if let Ok(bal) = sqlx::query!(
                r#"
                SELECT calculate_balance($1, $2);"#,
                user_id.to_string(),
                rec.currency.to_string()
            )
            .fetch_one(&mut *db)
            .await
            {
                details.insert(rec.currency, bal.calculate_balance.unwrap_or(0));
            }
        }

        Ok(details)
    }

    pub async fn reserve_by_asset(
        &self,
        user_uuid: Uuid,
        quantity: std::num::NonZeroU32,
        currency: &str,
    ) -> Result<ReserveOk, ReserveError> {
        let balance = self
            .calculate_balance_from_accounting(user_uuid, currency)
            .await?;

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

        let new_balance = self
            .calculate_balance_from_accounting(user_uuid, currency)
            .await?;
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
        match sqlx::query!(
            r#"
            INSERT INTO users (name, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            name,
            email,
            password_hash.as_bytes(),
        )
        .fetch_one(&self.db())
        .await
        {
            Ok(record) => Ok(record.id),
            Err(err) => Err(match err {
                sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
                    CreateUserError::EmailUniqueViolation(err)
                }
                _ => CreateUserError::Sqlx(err),
            }),
        }
    }

    pub async fn fetch_user_details(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<UserDetails, UserDetailsError> {
        let mut dtx = self.db.begin().await?;
        let _ = (*dtx).execute("SET TRANSACTION READ ONLY").await?;

        let rec = sqlx::query!(
            r#"SELECT id, name, role as "role: String"
            FROM users
            WHERE id = $1"#,
            user_id
        )
        .fetch_one(&mut *dtx)
        .await?;

        let mut addrs = sqlx::query!(
            r#"SELECT *
            FROM user_addresses
            WHERE user_id = $1"#,
            user_id
        )
        .fetch_all(&mut *dtx)
        .await?;

        let mut accounts = HashMap::new();
        for rec in addrs {
            let entry = accounts
                .entry(rec.currency.clone())
                .or_insert(UserAccountDetails {
                    amount: Default::default(),
                    deposit_address: None,
                    withrawal_addresses: vec![],
                });

            if rec.kind == "deposit" {
                entry.deposit_address.replace(rec.address_text.clone());
            } else {
                entry.withrawal_addresses.push(UserWalletAddr {
                    text: rec.address_text,
                    currency: rec.currency,
                    kind: rec.kind,
                });
            }
        }

        let details = UserDetails {
            name: rec.name,
            id: rec.id,
            role: rec.role,
            accounts,
            portfolio: UserPortfolio { value: 0 },
        };

        dtx.commit().await?;

        Ok(details)
    }

    pub async fn fetch_user_account(
        &self,
        user_id: Uuid,
        asset: Asset,
    ) -> Result<Option<UserAccount>, sqlx::Error> {
        let rec = match sqlx::query!(
            r#"SELECT id FROM accounts
            WHERE currency = $1
                AND source_type = 'user'
                AND source_id = $2"#,
            asset.to_string(),
            user_id.to_string()
        )
        .fetch_one(&self.db)
        .await
        {
            Ok(rec) => rec,
            Err(sqlx::Error::RowNotFound) => return Ok(None),
            Err(err) => return Err(err),
        };

        Ok(Some(UserAccount {}))
    }
}

#[cfg(test)]
mod test {
    use crate::spawn_trading_engine::spawn_trading_engine;

    use super::*;

    async fn make_app_cx_fixture(db: sqlx::PgPool) -> AppCx {
        let config = Configuration::load_from_toml("");
        let (te_tx, te_handle) = spawn_trading_engine(&config, db.clone())
            .init_from_db(db.clone())
            .await
            .unwrap();
        AppCx::new(te_tx, BitcoinRpcClient::new_mock(), db, config)
    }

    #[sqlx::test(migrations = "../../migrations")]
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

    #[sqlx::test(migrations = "../../migrations")]
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

        let balance = app_cx
            .calculate_balance_from_accounting(user_uuid, "BTC")
            .await
            .unwrap();

        assert_eq!(
            balance,
            NonZeroU64::new(total_credits as u64),
            "Expected balance does not match calculated balance: user={user_uuid} balance={balance:?} expected={total_credits:?}"
        );
    }
}
