use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::AssetPairRow;
use crate::asset_pair::BaseQuote;
use crate::decimal::Decimal;
use crate::decimal::dec;
use crate::orderbook::Orderbook;
use crate::svc::ap_actor::ApState;
use crate::svc::ap_actor::Envelope;
use crate::svc::ap_actor::asset_processor_loop;
use common_core::web::middleware::clerk::ClerkUserId;
use tokio::sync::mpsc;

mod test_ap_actor_batch_cancel_by_userref;
mod test_ap_actor_cancel_msgout_verification;
mod test_ap_actor_gtd_expiry;
mod test_ap_actor_lifecycle;
mod test_ap_actor_order_cancellation;
mod test_ap_actor_order_placement;
mod test_ap_actor_partial_fill_cancel;
mod test_ap_actor_price_validation;
mod test_ap_actor_settlement;
mod test_ap_actor_tracking_cleanup;

/// Test fixture data returned by setup
struct TestFixture {
    symbol_vocabulary: SymbolVocabulary,
    asset_pair_row: AssetPairRow,
    user_id: ClerkUserId,
    usd_account_id: i32,
    btc_account_id: i32,
    btc_usd: BaseQuote,
    exchange_usd_account_id: i32,
    exchange_btc_account_id: i32,
    ap_sender: mpsc::Sender<Envelope>,
}

async fn test_ap_actor_fixture(pg_pool: &sqlx::PgPool) -> TestFixture {
    // Create symbol vocabulary
    let symbol_vocabulary = SymbolVocabulary::from_iter(vec!["BTC".into(), "USD".into()]);

    // Get BTC/USD asset pair (created by migration)
    let asset_pair_row = sqlx::query_as!(
        AssetPairRow,
        r#"
            SELECT id, base_asset, quote_asset, status,
                   min_order_size, max_order_size,
                   price_tick_size, quantity_tick_size,
                   created_at, updated_at
            FROM t_trading_asset_pairs
            WHERE base_asset = $1 AND quote_asset = $2
            "#,
        "BTC",
        "USD"
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    // Create test user
    let test_user_id = "user-test123";
    sqlx::query!(
        "INSERT INTO t_user_data (id, name, email, password_hash) VALUES ($1, $2, $3, $4)",
        test_user_id,
        "Test User",
        "test@example.com",
        &[] as &[u8] // empty password hash for test
    )
    .execute(pg_pool)
    .await
    .unwrap();

    let user_id = ClerkUserId(test_user_id.to_string());

    // Create user money accounts
    let usd_account_id = sqlx::query_scalar!(
        "INSERT INTO t_money_accounts (currency, source_type, source_id) VALUES ($1, $2, $3) RETURNING id",
        "USD",
        "user",
        format!("user:{}", test_user_id)
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    let btc_account_id = sqlx::query_scalar!(
        "INSERT INTO t_money_accounts (currency, source_type, source_id) VALUES ($1, $2, $3) RETURNING id",
        "BTC",
        "user",
        format!("user:{}", test_user_id)
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    // Get exchange accounts
    let exchange_usd_account_id = sqlx::query_scalar!(
        "SELECT id FROM t_money_accounts WHERE source_id = $1 AND currency = $2",
        "fiat:exchange",
        "USD"
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    let exchange_btc_account_id = sqlx::query_scalar!(
        "SELECT id FROM t_money_accounts WHERE source_id = $1 AND currency = $2",
        "crypto:bitcoin",
        "BTC"
    )
    .fetch_one(pg_pool)
    .await
    .unwrap();

    // Give test user initial balance: 100,000 USD
    sqlx::query!(
        r#"
            INSERT INTO t_account_tx_journal
            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            VALUES ($1, $2, $3, $4::numeric, $5, $6)
            "#,
        usd_account_id,
        exchange_usd_account_id,
        "USD",
        dec!(100_000), // 100k USD
        "test_deposit",
        uuid::Uuid::new_v4().to_string()
    )
    .execute(pg_pool)
    .await
    .unwrap();

    // Give test user initial balance: 10 BTC
    sqlx::query!(
        r#"
            INSERT INTO t_account_tx_journal
            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            VALUES ($1, $2, $3, $4::numeric, $5, $6)
            "#,
        btc_account_id,
        exchange_btc_account_id,
        "BTC",
        Decimal::from(10), // 10 BTC
        "test_deposit",
        uuid::Uuid::new_v4().to_string()
    )
    .execute(pg_pool)
    .await
    .unwrap();

    // Spawn actor
    let (actor_tx, actor_rx) = mpsc::channel(16);
    let proc = ApState {
        pg_pool: pg_pool.clone(),
        orderbook: Orderbook::new_empty(),
        tracking: Default::default(),
        asset_pair_row: asset_pair_row.clone(),
        symbol_vocabulary: symbol_vocabulary.clone(),
        expiry_queue: vec![],
    };
    tokio::spawn(asset_processor_loop(actor_rx, proc));

    let btc_usd = (
        AssetCode::from_str_and_vocabulary("BTC", &symbol_vocabulary).unwrap(),
        AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap(),
    );

    TestFixture {
        symbol_vocabulary,
        asset_pair_row,
        user_id: user_id,
        btc_usd,
        usd_account_id,
        btc_account_id,
        exchange_usd_account_id,
        exchange_btc_account_id,
        ap_sender: actor_tx,
    }
}
