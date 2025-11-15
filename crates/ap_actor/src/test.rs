use crate::proc::Envelope;
use crate::proc::launch_processors_for_pairs;
use matching_engine::asset_code::AssetCode;
use matching_engine::asset_code::SymbolVocabulary;
use matching_engine::asset_pair::AssetPairRow;
use matching_engine::asset_pair::BaseQuote;
use matching_engine::decimal::Decimal;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_ticket::OrderTicketBuilder;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;
use matching_engine::time::Time;
use sqlx::types::time::PrimitiveDateTime;
use tokio::sync::mpsc;

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
            "USD"
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        let exchange_btc_account_id = sqlx::query_scalar!(
            "SELECT id FROM t_money_accounts WHERE crypto_source = $1 AND currency = $2",
            "bitcoin",
            "BTC"
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
            "USD",
            user_data_id
        )
        .fetch_one(pg_pool)
        .await
        .unwrap();

        let btc_account_id = sqlx::query_scalar!(
            "INSERT INTO t_money_accounts (currency, user_id) VALUES ($1, $2) RETURNING id",
            "BTC",
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
            "USD",
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
            "BTC",
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

    pub async fn balance(&self, pg_pool: &sqlx::PgPool, account_id: i32) -> Decimal {
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
    pub ap_sender: mpsc::Sender<Envelope>,
}

pub async fn test_ap_actor_fixture(pg_pool: &sqlx::PgPool) -> TestFixture {
    let asset_pair_row = sqlx::query_as!(
        AssetPairRow,
        r#"
            SELECT * FROM t_trading_asset_pairs
            WHERE base_asset = $1 AND quote_asset = $2
            "#,
        "BTC",
        "ZUSD"
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    let exchange_usd_account_id = sqlx::query_scalar!(
        "SELECT id FROM t_money_accounts WHERE fiat_source = $1 AND currency = $2",
        "exchange",
        "USD"
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    let exchange_btc_account_id = sqlx::query_scalar!(
        "SELECT id FROM t_money_accounts WHERE crypto_source = $1 AND currency = $2",
        "bitcoin",
        "BTC"
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    let (symbol_vocabulary, ap_info) = launch_processors_for_pairs(
        vec![asset_pair_row.clone()],
        pg_pool.clone(),
    )
    .await
    .unwrap();

    let ap_sender = ap_info.first().unwrap().mpsc_sender.clone();

    let btc_usd = (
        AssetCode::from_str_and_vocabulary("BTC", &symbol_vocabulary).unwrap(),
        AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap(),
    );

    TestFixture {
        symbol_vocabulary,
        asset_pair_row,
        btc_usd,
        exchange_usd_account_id,
        exchange_btc_account_id,
        ap_sender,
    }
}

pub fn non_zero(amount: Decimal) -> matching_engine::decimal::NonZeroDecimal {
    matching_engine::decimal::NonZeroDecimal::new(amount)
        .expect("quantity must be positive for tests")
}

pub fn price_from_str(input: &str) -> Price {
    use std::str::FromStr;
    Price::from_str(input).expect("test price must parse")
}

pub fn price_absolute(amount: Decimal) -> Price {
    Price {
        prefix: None,
        amount,
        is_percentage: false,
    }
}

pub fn limit_builder(
    side: OrderSide,
    price: impl Into<Price>,
    volume: Decimal,
) -> OrderTicketBuilder {
    OrderTicket::builder(OrderType::Limit, side, price.into())
        .quantity(non_zero(volume))
        .volume(volume)
}

pub fn market_builder(
    side: OrderSide,
    reference_price: impl Into<Price>,
    volume: Decimal,
) -> OrderTicketBuilder {
    OrderTicket::builder(OrderType::Market, side, reference_price.into())
        .quantity(non_zero(volume))
        .volume(volume)
}

pub fn stop_loss_builder(
    side: OrderSide,
    trigger_price: impl Into<Price>,
    volume: Decimal,
) -> OrderTicketBuilder {
    OrderTicket::builder(OrderType::StopLoss, side, trigger_price.into())
        .quantity(non_zero(volume))
        .volume(volume)
}

pub fn stop_loss_limit_builder(
    side: OrderSide,
    trigger_price: impl Into<Price>,
    limit_price: impl Into<Price>,
    volume: Decimal,
) -> OrderTicketBuilder {
    OrderTicket::builder(OrderType::StopLossLimit, side, trigger_price.into())
        .quantity(non_zero(volume))
        .volume(volume)
        .secondary_price(limit_price.into())
}

pub fn take_profit_builder(
    side: OrderSide,
    trigger_price: impl Into<Price>,
    volume: Decimal,
) -> OrderTicketBuilder {
    OrderTicket::builder(OrderType::TakeProfit, side, trigger_price.into())
        .quantity(non_zero(volume))
        .volume(volume)
}

pub fn take_profit_limit_builder(
    side: OrderSide,
    trigger_price: impl Into<Price>,
    limit_price: impl Into<Price>,
    volume: Decimal,
) -> OrderTicketBuilder {
    OrderTicket::builder(OrderType::TakeProfitLimit, side, trigger_price.into())
        .quantity(non_zero(volume))
        .volume(volume)
        .secondary_price(limit_price.into())
}

pub fn expiry_in_seconds(seconds: u64) -> Time {
    Time::Scheduled(seconds)
}
