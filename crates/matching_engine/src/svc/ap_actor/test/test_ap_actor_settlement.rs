use tokio::sync::oneshot;

use crate::asset_code::AssetCode;
use crate::decimal::Decimal;
use crate::decimal::dec;
use crate::order_uuid::OrderUuid;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::order_management::PlaceOrderArgs;
use common_core::web::middleware::clerk::ClerkUserId;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_settlement(pg_pool: sqlx::PgPool) {
    let TestFixture {
        symbol_vocabulary,
        user_id: user1_id,
        usd_account_id: user1_usd_account_id,
        btc_account_id: user1_btc_account_id,
        exchange_usd_account_id,
        exchange_btc_account_id,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user2_test_id = "user-test456";
    sqlx::query!(
        "INSERT INTO t_user_data (id, name, email, password_hash) VALUES ($1, $2, $3, $4)",
        user2_test_id,
        "Test User 2",
        "test2@example.com",
        &[] as &[u8]
    )
    .execute(&pg_pool)
    .await
    .unwrap();

    let user2_id = ClerkUserId(user2_test_id.to_string());

    let user2_usd_account_id = sqlx::query_scalar!(
        "INSERT INTO t_money_accounts (currency, source_type, source_id) VALUES ($1, $2, $3) RETURNING id",
        "USD",
        "user",
        format!("user:{}", user2_test_id)
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    let user2_btc_account_id = sqlx::query_scalar!(
        "INSERT INTO t_money_accounts (currency, source_type, source_id) VALUES ($1, $2, $3) RETURNING id",
        "BTC",
        "user",
        format!("user:{}", user2_test_id)
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
            INSERT INTO t_account_tx_journal
            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            VALUES ($1, $2, $3, $4::numeric, $5, $6)
            "#,
        user2_usd_account_id,
        exchange_usd_account_id,
        "USD",
        dec!(100_000),
        "test_deposit",
        uuid::Uuid::new_v4().to_string()
    )
    .execute(&pg_pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
            INSERT INTO t_account_tx_journal
            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
            VALUES ($1, $2, $3, $4::numeric, $5, $6)
            "#,
        user2_btc_account_id,
        exchange_btc_account_id,
        "BTC",
        dec!(10),
        "test_deposit",
        uuid::Uuid::new_v4().to_string()
    )
    .execute(&pg_pool)
    .await
    .unwrap();

    let sell_order_json = serde_json::json!({
        "nonce": 123456793,
        "type": "sell",
        "ordertype": "limit",
        "volume": "0.5",
        "displayvol": "0.5",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let sell_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: (
            AssetCode::from_str_and_vocabulary("BTC", &symbol_vocabulary).unwrap(),
            AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap(),
        ),
        user_id: user1_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: serde_json::from_value(sell_order_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, sell_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Sell order should be placed successfully, got: {:?}",
        resp
    );

    // Verify event sourcing entry exists for sell order
    let event_count = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM t_trading_event_source"#)
        .fetch_one(&pg_pool)
        .await
        .unwrap();
    assert!(
        event_count.unwrap() >= 1,
        "Event sourcing should record sell order"
    );

    let buy_order_json = serde_json::json!({
        "nonce": 123456794,
        "type": "buy",
        "ordertype": "market",
        "volume": "0.5",
        "displayvol": "0.5",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: (
            AssetCode::from_str_and_vocabulary("BTC", &symbol_vocabulary).unwrap(),
            AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap(),
        ),
        user_id: user2_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: serde_json::from_value(buy_order_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, buy_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Buy order should be placed successfully and match, got: {:?}",
        resp
    );

    // Verify both orders persisted to event source
    let total_events = sqlx::query_scalar!(r#"SELECT COUNT(*) FROM t_trading_event_source"#)
        .fetch_one(&pg_pool)
        .await
        .unwrap();
    assert!(
        total_events.unwrap() >= 2,
        "Should have at least 2 events (sell + buy orders)"
    );

    let settlement_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'trade_settlement'
            "#
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        settlement_count.unwrap_or(0),
        2,
        "Should have exactly 2 settlement transactions"
    );

    // Verify BTC transfer: exchange → user2 (buyer)
    // User2 is the buyer (market buy), so they should be CREDITED BTC
    let btc_settlement = sqlx::query!(
        r#"
            SELECT credit_account_id, debit_account_id, amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'trade_settlement'
            AND currency = 'BTC'
            "#
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        btc_settlement.credit_account_id, user2_btc_account_id,
        "Buyer (user2) should be CREDITED BTC"
    );
    assert_eq!(
        btc_settlement.debit_account_id, exchange_btc_account_id,
        "Exchange should be DEBITED BTC"
    );
    assert_eq!(
        btc_settlement.amount,
        dec!(0.5),
        "BTC settlement amount should be 0.5 BTC"
    );

    // Verify USD transfer: exchange → user1 (seller)
    // User1 is the seller (limit sell), so they should be CREDITED USD
    let usd_settlement = sqlx::query!(
        r#"
            SELECT credit_account_id, debit_account_id, amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'trade_settlement'
            AND currency = 'USD'
            "#
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        usd_settlement.credit_account_id, user1_usd_account_id,
        "Seller (user1) should be CREDITED USD"
    );
    assert_eq!(
        usd_settlement.debit_account_id, exchange_usd_account_id,
        "Exchange should be DEBITED USD"
    );
    assert_eq!(
        usd_settlement.amount,
        dec!(25_000),
        "USD settlement amount should be $25,000"
    );

    let user1_btc_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        user1_btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        user1_btc_balance,
        dec!(9.5),
        "User 1 BTC balance should be 9.5 BTC after selling 0.5 BTC, got: {}",
        user1_btc_balance
    );

    let user1_usd_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        user1_usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        user1_usd_balance,
        dec!(125_000),
        "User 1 USD balance should be $125,000 after receiving $25k from sale, got: {}",
        user1_usd_balance
    );

    let user2_btc_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        user2_btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        user2_btc_balance,
        dec!(10.5),
        "User 2 BTC balance should be 10.5 BTC after buying 0.5 BTC, got: {}",
        user2_btc_balance
    );

    let user2_usd_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        user2_usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        user2_usd_balance,
        dec!(75_000),
        "User 2 USD balance should be $75,000 after paying $25k for BTC, got: {}",
        user2_usd_balance
    );

    // Verify exchange accounts were properly debited in settlement
    let exchange_btc_debits = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM t_account_tx_journal
            WHERE debit_account_id = $1
            AND transaction_type = 'trade_settlement'
            "#,
        exchange_btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        exchange_btc_debits.unwrap(),
        dec!(0.5),
        "Exchange should have debited exactly 0.5 BTC to buyer"
    );

    let exchange_usd_debits = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM t_account_tx_journal
            WHERE debit_account_id = $1
            AND transaction_type = 'trade_settlement'
            "#,
        exchange_usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        exchange_usd_debits.unwrap(),
        dec!(25_000),
        "Exchange should have debited exactly $25,000 to seller"
    );
}
