use ap_actor::order_management::CancelOrderBy;
use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::CancelOrderByArgs;
use ap_actor::proc::MsgError;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::orderbook::TimeInForce;
use matching_engine::price::Price;
use std::time;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_gtd_order_expiry(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user = TestUser::random().create(&pg_pool).await;

    let initial_usd_balance = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        initial_usd_balance,
        dec!(100_000),
        "Initial USD balance should be $100k"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 2;
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let gtd_buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: order_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, gtd_buy_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "GTD buy order should be placed successfully, got: {:?}",
        resp
    );

    let balance_after_place = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        balance_after_place,
        dec!(95_000),
        "User USD balance should be $95k after reserving $5k for GTD order"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let expiry_refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'gtd_expiry_refund'
            AND credit_account_id = $1
            "#,
        user.usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        expiry_refund_count.unwrap_or(0),
        1,
        "Should have exactly 1 gtd_expiry_refund transaction"
    );

    let refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'gtd_expiry_refund'
            AND credit_account_id = $1
            AND currency = 'USD'
            "#,
        user.usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(refund_amount, dec!(5_000), "Refund amount should be $5,000");

    let final_balance = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        final_balance,
        dec!(100_000),
        "User USD balance should be fully restored to $100k after expiry"
    );

    let cancel_expired_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_expired_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(MsgError::OrderNotFound)),
        "Trying to cancel already-expired order should return OrderNotFound, got: {:?}",
        resp
    );

    let initial_btc_balance = user.balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        initial_btc_balance,
        dec!(10),
        "Initial BTC balance should be 10 BTC"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 2;
    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let gtd_sell_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: sell_order_uuid.clone(),
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
        .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, gtd_sell_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "GTD sell order should be placed successfully, got: {:?}",
        resp
    );

    let btc_balance_after_place = user.balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_balance_after_place,
        dec!(9.5),
        "User BTC balance should be 9.5 BTC after reserving 0.5 BTC"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let btc_refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'gtd_expiry_refund'
            AND credit_account_id = $1
            AND currency = 'BTC'
            "#,
        user.btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        btc_refund_amount,
        dec!(0.5),
        "BTC refund amount should be 0.5 BTC"
    );

    let btc_balance_after_expiry = user.balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_balance_after_expiry,
        dec!(10),
        "User BTC balance should be fully restored to 10 BTC after expiry"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 10;
    let manual_uuid = OrderUuid(uuid::Uuid::new_v4());
    let manual_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: manual_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(45000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .volume(dec!(0.2))
        .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, manual_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order with long GTD should be placed successfully"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let cancel_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(manual_uuid),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Manual cancel before expiry should succeed"
    );

    let manual_refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
            AND credit_account_id = $1
            AND currency = 'USD'
            "#,
        user.usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert!(
        manual_refund_count.unwrap_or(0) > 0,
        "Should have cancel_refund transaction for manual cancellation"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 2;
    let order_a_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_a = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: order_a_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(49000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.15)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.15)).unwrap())
        .volume(dec!(0.15))
        .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_a)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced { .. })));

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 5;
    let order_b_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_b = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: order_b_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(47000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.25)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.25)).unwrap())
        .volume(dec!(0.25))
        .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_b)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced { .. })));

    let balance_with_two_orders = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        balance_with_two_orders,
        dec!(80_900),
        "Balance should reflect both orders reserved"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let balance_after_first_expiry = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        balance_after_first_expiry,
        dec!(88_250),
        "Balance should reflect order A refunded but order B still reserved"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let final_balance_both_expired = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        final_balance_both_expired,
        dec!(100_000),
        "Balance should be fully restored after both orders expire"
    );
}
