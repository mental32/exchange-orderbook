use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::asset_code::AssetCode;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;
use std::str::FromStr;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_settlement(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        symbol_vocabulary,
        exchange_usd_account_id,
        exchange_btc_account_id,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let sell_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: (
            AssetCode::from_str_and_vocabulary("BTC", &symbol_vocabulary).unwrap(),
            AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap(),
        ),
        user_id: user1.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Sell,
            Price {
                prefix: None,
                amount: dec!(50000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
        .volume(dec!(0.5))
        .build()
        .unwrap(),
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

    let buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: (
            AssetCode::from_str_and_vocabulary("BTC", &symbol_vocabulary).unwrap(),
            AssetCode::from_str_and_vocabulary("USD", &symbol_vocabulary).unwrap(),
        ),
        user_id: user2.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
            OrderType::Market,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
        .volume(dec!(0.5))
        .build()
        .unwrap(),
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
        btc_settlement.credit_account_id, user2.btc_account_id,
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
        usd_settlement.credit_account_id, user1.usd_account_id,
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

    let user1_btc_balance = user1.balance(&pg_pool, user1.btc_account_id).await;

    assert_eq!(
        user1_btc_balance,
        dec!(9.5),
        "User 1 BTC balance should be 9.5 BTC after selling 0.5 BTC, got: {}",
        user1_btc_balance
    );

    let user1_usd_balance = user1.balance(&pg_pool, user1.usd_account_id).await;

    assert_eq!(
        user1_usd_balance,
        dec!(125_000),
        "User 1 USD balance should be $125,000 after receiving $25k from sale, got: {}",
        user1_usd_balance
    );

    let user2_btc_balance = user2.balance(&pg_pool, user2.btc_account_id).await;

    assert_eq!(
        user2_btc_balance,
        dec!(10.5),
        "User 2 BTC balance should be 10.5 BTC after buying 0.5 BTC, got: {}",
        user2_btc_balance
    );

    let user2_usd_balance = user2.balance(&pg_pool, user2.usd_account_id).await;

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
