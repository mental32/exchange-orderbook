use crate::proc::launch_processors_for_pairs;
use crate::proc_router::ProcRouter;
use matching_engine::asset_code::AssetCode;
use matching_engine::asset_code::SymbolVocabulary;
use matching_engine::asset_pair::AssetPairRow;
use matching_engine::asset_pair::BaseQuote;
use matching_engine::decimal::Decimal;
use matching_engine::decimal::dec;
use sqlx::types::Json;
use std::ops::DerefMut;

pub const ZUSD: &str = "USD";
pub const XBTC: &str = "BTC";

pub struct TestUser {
    pub user_id: i32,
    pub usd_account_id: i32,
    pub btc_account_id: i32,
    pub user_id_str: String,
    pub name: String,
    pub email: String,
}

impl TestUser {
    pub fn random() -> Self {
        let id = uuid::Uuid::new_v4();
        Self {
            user_id_str: format!("user-test-{}", id.as_u128()),
            name: format!("Test User {}", id.as_u128()),
            email: format!("test{}@example.com", id.as_u128()),
            user_id: 0,
            usd_account_id: 0,
            btc_account_id: 0,
        }
    }

    pub async fn create(mut self, pg_pool: &sqlx::PgPool) -> Self {
        let exchange_usd_account_id = sqlx::query_scalar!(
            "SELECT id FROM t_money_accounts WHERE fiat_source = $1 AND currency = $2",
            "exchange",
            ZUSD
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        let exchange_btc_account_id = sqlx::query_scalar!(
            "SELECT id FROM t_money_accounts WHERE crypto_source = $1 AND currency = $2",
            "bitcoin",
            XBTC
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        let user_data_id = sqlx::query_scalar!(
            "INSERT INTO t_user_data (clerk, tier, name, email, password_hash) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            self.user_id_str,
            1,
            self.name,
            self.email,
            &[] as &[u8]
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        let usd_account_id = sqlx::query_scalar!(
            "INSERT INTO t_money_accounts (currency, user_id) VALUES ($1, $2) RETURNING id",
            ZUSD,
            user_data_id
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        let btc_account_id = sqlx::query_scalar!(
            "INSERT INTO t_money_accounts (currency, user_id) VALUES ($1, $2) RETURNING id",
            XBTC,
            user_data_id
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        sqlx::query!(
            r#"
                INSERT INTO t_account_tx_journal
                (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                VALUES ($1, $2, $3, $4::numeric, $5, $6)
                "#,
            usd_account_id,
            exchange_usd_account_id,
            ZUSD,
            dec!(100_000),
            "test_deposit",
            uuid::Uuid::new_v4().to_string()
        )
        .execute(pg_pool)
        .await
        .unwrap();

        sqlx::query!(
            r#"
                INSERT INTO t_account_tx_journal
                (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                VALUES ($1, $2, $3, $4::numeric, $5, $6)
                "#,
            btc_account_id,
            exchange_btc_account_id,
            XBTC,
            Decimal::from(10),
            "test_deposit",
            uuid::Uuid::new_v4().to_string()
        )
        .execute(pg_pool)
        .await
        .unwrap();

        self.user_id = user_data_id;
        self.usd_account_id = usd_account_id;
        self.btc_account_id = btc_account_id;
        self
    }

    pub async fn compute_balance(&self, pg_pool: &sqlx::PgPool, account_id: i32) -> Decimal {
        sqlx::query_scalar!(
            r#"
                SELECT COALESCE(
                    (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                    0
                ) - COALESCE(
                    (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                    0
                ) as "balance!"
                "#,
            account_id
        )
        .fetch_one(pg_pool)
        .await
        .unwrap()
    }
}

pub struct TestFixture {
    pub symbol_vocabulary: SymbolVocabulary,
    pub asset_pair_row: AssetPairRow,
    pub btc_usd: BaseQuote,
    pub exchange_usd_account_id: i32,
    pub exchange_btc_account_id: i32,
    pub proc_router: ProcRouter,
}

pub async fn test_ap_actor_fixture(pg_pool: &sqlx::PgPool) -> TestFixture {
    let mut transaction = pg_pool.begin().await.unwrap();

    sqlx::query!("DELETE FROM t_trading_event_source")
        .execute(transaction.deref_mut())
        .await
        .unwrap();

    sqlx::query!("DELETE FROM t_account_tx_journal_outbox")
        .execute(transaction.deref_mut())
        .await
        .unwrap();

    let asset_pair_row = sqlx::query_as!(
        AssetPairRow,
        r#"
            SELECT * FROM t_trading_asset_pairs
            WHERE base_asset = $1 AND quote_asset = $2
            "#,
        XBTC,
        ZUSD
    )
    .fetch_one(transaction.deref_mut())
    .await
    .unwrap();

    let exchange_usd_account_id = sqlx::query_scalar!(
        "SELECT id FROM t_money_accounts WHERE fiat_source = $1 AND currency = $2",
        "exchange",
        ZUSD
    )
    .fetch_one(transaction.deref_mut())
    .await
    .unwrap();

    let exchange_btc_account_id = sqlx::query_scalar!(
        "SELECT id FROM t_money_accounts WHERE crypto_source = $1 AND currency = $2",
        "bitcoin",
        XBTC
    )
    .fetch_one(transaction.deref_mut())
    .await
    .unwrap();
    transaction.commit().await.unwrap();

    let (symbol_vocabulary, asset_processors) =
        launch_processors_for_pairs(vec![asset_pair_row.clone()], pg_pool.clone())
            .await
            .unwrap();

    let proc_router = ProcRouter::new(symbol_vocabulary.clone(), asset_processors);

    let btc_usd = (
        AssetCode::from_str_and_vocabulary(XBTC, &symbol_vocabulary).unwrap(),
        AssetCode::from_str_and_vocabulary(ZUSD, &symbol_vocabulary).unwrap(),
    );

    TestFixture {
        symbol_vocabulary,
        asset_pair_row,
        btc_usd,
        exchange_usd_account_id,
        exchange_btc_account_id,
        proc_router,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventSourceRow {
    pub id: i64,
    pub jstr: Json<serde_json::Value>,
    pub quote_asset: String,
    pub base_asset: String,
}

pub async fn t_trading_event_source(pg_pool: &sqlx::PgPool) -> Vec<EventSourceRow> {
    sqlx::query_as!(
        EventSourceRow,
        "SELECT id, jstr, quote_asset, base_asset FROM t_trading_event_source"
    )
    .fetch_all(pg_pool)
    .await
    .unwrap()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AccountTxJournal {
    pub id: i64,
    pub credit_account_id: i64,
    pub debit_account_id: i64,
    pub currency: String,
    pub amount: matching_engine::decimal::Decimal,
    pub transaction_type: String,
}

pub async fn t_account_tx_journal(pg_pool: &sqlx::PgPool) -> Vec<AccountTxJournal> {
    sqlx::query_as!(AccountTxJournal, "SELECT id, credit_account_id, debit_account_id, currency, amount, transaction_type FROM t_account_tx_journal")
        .fetch_all(pg_pool)
        .await
        .unwrap()
}
