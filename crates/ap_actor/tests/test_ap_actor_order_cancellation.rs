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
use matching_engine::price::Price;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_order_cancellation(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user = TestUser::random().create(&pg_pool).await;

    let initial_balance = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        initial_balance,
        dec!(100_000),
        "Initial USD balance should be $100k, got: {}",
        initial_balance
    );

    // Place limit buy order: 0.1 BTC @ $50,000 = $5,000 reserved
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let limit_buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: order_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, limit_buy_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Limit buy order should be placed successfully, got: {:?}",
        resp
    );

    let balance_after_place = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after_place,
        dec!(95_000),
        "User USD balance should be reduced by $5k (0.1 BTC * $50k), got: {}",
        balance_after_place
    );

    // Cancel the order by TxId
    let cancel_order_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::TxId(order_uuid.clone()),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_order_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order should be cancelled successfully, got: {:?}",
        resp
    );

    // Verify refund transaction was created
    let refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
            AND credit_account_id = $1
            "#,
        user.usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        refund_count.unwrap_or(0),
        1,
        "Should have exactly 1 cancel_refund transaction"
    );

    // Verify refund amount is correct ($5,000)
    let refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
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

    assert_eq!(
        refund_amount,
        dec!(5_000),
        "Refund amount should be $5,000 (0.1 BTC * $50k), got: {}",
        refund_amount
    );

    let final_balance = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        final_balance,
        dec!(100_000),
        "User USD balance should be fully restored to $100k after cancellation, got: {}",
        final_balance
    );

    // Test error case: try to cancel the same order again
    let cancel_again_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::TxId(order_uuid),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_again_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(MsgError::OrderNotFound)),
        "Cancelling already-cancelled order should return OrderNotFound, got: {:?}",
        resp
    );

    let initial_btc_balance = user.balance(&pg_pool, user.btc_account_id).await;
    assert_eq!(
        initial_btc_balance,
        dec!(10),
        "Initial BTC balance should be 10 BTC, got: {}",
        initial_btc_balance
    );

    // Place limit sell order: 0.5 BTC @ $50,000 (reserves 0.5 BTC)
    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let limit_sell_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: sell_order_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Sell,
            Price {
                prefix: None,
                amount: dec!(50_000),
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
        ap_sender.send((resp_tx, limit_sell_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Limit sell order should be placed successfully, got: {:?}",
        resp
    );

    let btc_balance_after_sell = user.balance(&pg_pool, user.btc_account_id).await;
    assert_eq!(
        btc_balance_after_sell,
        dec!(9.5),
        "User BTC balance should be reduced by 0.5 BTC, got: {}",
        btc_balance_after_sell
    );

    // Cancel the sell order
    let cancel_sell_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::TxId(sell_order_uuid),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_sell_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Sell order should be cancelled successfully, got: {:?}",
        resp
    );

    // Verify BTC refund transaction
    let btc_refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
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
        "BTC refund amount should be 0.5 BTC, got: {}",
        btc_refund_amount
    );

    let btc_balance_after_cancel = user.balance(&pg_pool, user.btc_account_id).await;
    assert_eq!(
        btc_balance_after_cancel,
        dec!(10),
        "User BTC balance should be fully restored to 10 BTC after cancellation, got: {}",
        btc_balance_after_cancel
    );

    // Section 3: Cancel by ClientOrderId
    let cl_ord_id_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_with_cl_ord_id = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: cl_ord_id_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(45_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .volume(dec!(0.2))
        .cl_ord_id("test-order-cl-123".to_owned())
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, order_with_cl_ord_id))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order with cl_ord_id should be placed successfully, got: {:?}",
        resp
    );

    let usd_balance_before_cl_cancel = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_before_cl_cancel,
        dec!(91_000),
        "User USD balance should be $91k after reserving $9k (0.2 BTC * $45k), got: {}",
        usd_balance_before_cl_cancel
    );

    // Cancel by ClientOrderId
    let cancel_by_cl_ord_id_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::ClientOrderId("test-order-cl-123".to_owned()),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_by_cl_ord_id_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Cancel by ClientOrderId should succeed, got: {:?}",
        resp
    );

    let usd_balance_after_cl_cancel = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_after_cl_cancel,
        dec!(100_000),
        "User USD balance should be restored to $100k after cancel by cl_ord_id, got: {}",
        usd_balance_after_cl_cancel
    );

    // Section 4: Cancel by Userref
    let userref_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_with_userref = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: userref_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(48_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.3)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.3)).unwrap())
        .volume(dec!(0.3))
        .userref(42)
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_with_userref)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order with userref should be placed successfully, got: {:?}",
        resp
    );

    let usd_balance_before_userref_cancel = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_before_userref_cancel,
        dec!(85_600),
        "User USD balance should be $85,600 after reserving $14,400 (0.3 BTC * $48k), got: {}",
        usd_balance_before_userref_cancel
    );

    // Cancel by Userref
    let cancel_by_userref_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::Userref(42),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_by_userref_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Cancel by Userref should succeed, got: {:?}",
        resp
    );

    let usd_balance_after_userref_cancel = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_after_userref_cancel,
        dec!(100_000),
        "User USD balance should be restored to $100k after cancel by userref, got: {}",
        usd_balance_after_userref_cancel
    );

    // Section 5: NoOpenPositions error
    let user_no_orders = TestUser::random().create(&pg_pool).await.user_id;

    // Try to cancel order for user with no positions
    let cancel_no_positions_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user_no_orders,
        cancel_order_by: CancelOrderBy::TxId(OrderUuid(uuid::Uuid::new_v4())),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_no_positions_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(MsgError::NoOpenPositions)),
        "Cancelling order for user with no positions should return NoOpenPositions, got: {:?}",
        resp
    );

    // Section 6: Multiple orders - cancel specific one
    // Place first order with userref=100
    let order_a_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_a = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: order_a_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(49_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.15)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.15)).unwrap())
        .volume(dec!(0.15))
        .userref(100)
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_a)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order A should be placed successfully, got: {:?}",
        resp
    );

    // Place second order with userref=200
    let order_b_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_b = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: order_b_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(47_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.25)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.25)).unwrap())
        .volume(dec!(0.25))
        .userref(200)
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_b)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order B should be placed successfully, got: {:?}",
        resp
    );

    let usd_balance_with_two_orders = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_with_two_orders,
        dec!(80_900),
        "User USD balance should be $80,900 after reserving $19,100 for two orders, got: {}",
        usd_balance_with_two_orders
    );

    // Cancel only order A (userref=100)
    let cancel_order_a_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::Userref(100),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_order_a_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order A should be cancelled successfully, got: {:?}",
        resp
    );

    let usd_balance_after_cancel_a = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_after_cancel_a,
        dec!(88_250),
        "User USD balance should be $88,250 after cancelling order A ($7,350 refund), got: {}",
        usd_balance_after_cancel_a
    );

    // Cancel order B (userref=200)
    let cancel_order_b_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id,
        cancel_order_by: CancelOrderBy::Userref(200),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_order_b_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order B should be cancelled successfully, got: {:?}",
        resp
    );

    let usd_balance_final = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        usd_balance_final,
        dec!(100_000),
        "User USD balance should be fully restored to $100k after cancelling both orders, got: {}",
        usd_balance_final
    );
}
