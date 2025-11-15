use ap_actor::order_management::CancelOrderBy;
use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::CancelOrderByArgs;
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
async fn test_ap_actor_tracking_cleanup(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Step 1: Place an order
    let order1_uuid = OrderUuid(uuid::Uuid::new_v4());

    let order1_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: order1_uuid.clone(),
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
        .userref(777)
        .build()
        .unwrap(),
    });

    let resp1 = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order1_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp1, Ok(MsgOut::OrderPlaced));

    let balance_after_order1 = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after_order1,
        dec!(95_000),
        "Should have $95k after reserving $5k"
    );

    // Step 2: Cancel the order
    let cancel_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order1_uuid.clone()),
    });

    let cancel_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    assert_eq!(
        cancel_resp,
        Ok(MsgOut::OrderCancelled {
            success: vec![order1_uuid],
            failed: vec![]
        })
    );

    let balance_after_cancel = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after_cancel,
        dec!(100_000),
        "Balance should be restored to $100k"
    );

    // Step 4: CRITICAL TEST - Place another order with the same userref
    // If tracking wasn't cleaned up properly, this would fail because:
    // - The old order would still be in tracking
    // - Cancelling by userref would try to cancel the old order
    // - orderbook.get() would panic because the old order no longer exists in orderbook
    let order2_uuid = OrderUuid(uuid::Uuid::new_v4());

    let order2_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: order2_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(48000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .volume(dec!(0.2))
        .userref(777)
        .build()
        .unwrap(),
    });

    let resp2 = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order2_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp2, Ok(MsgOut::OrderPlaced));

    // Step 5: Cancel by userref - should ONLY cancel order2, not the ghost of order1
    let cancel_by_userref_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::Userref(777),
    });

    let cancel_userref_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_by_userref_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };

    // This is the critical assertion - if tracking cleanup failed, this would panic
    // because it would try to access order1 from the orderbook (which was removed)
    assert_eq!(
        cancel_userref_resp,
        Ok(MsgOut::OrderCancelled {
            success: vec![order2_uuid],
            failed: vec![]
        })
    );

    let final_balance = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        final_balance,
        dec!(100_000),
        "Final balance should be $100k after all cancellations"
    );
}
