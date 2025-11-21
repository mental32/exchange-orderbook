use ap_actor::proc_router::CancelOrderBy;
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

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_tracking_cleanup(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Step 1: Place an order
    let order1_uuid = OrderUuid(uuid::Uuid::new_v4());

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order1_uuid.clone(),
            OrderTicket::builder(
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
            .userref(777)
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    let balance_after_order1 = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after_order1,
        dec!(95_000),
        "Should have $95k after reserving $5k"
    );

    // Step 2: Cancel the order
    let cancel_resp = proc_router
        .cancel_order(
            CancelOrderBy::TxId(order1_uuid.clone()),
            user.user_id.clone(),
        )
        .await
        .unwrap();

    assert_eq!(cancel_resp, (vec![order1_uuid.0], vec![]));

    let balance_after_cancel = user.compute_balance(&pg_pool, user.usd_account_id).await;
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

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order2_uuid.clone(),
            OrderTicket::builder(
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
            .userref(777)
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    // Step 5: Cancel by userref - should ONLY cancel order2, not the ghost of order1
    let cancel_userref_resp = proc_router
        .cancel_order(CancelOrderBy::Userref(777), user.user_id.clone())
        .await
        .unwrap();

    // This is the critical assertion - if tracking cleanup failed, this would panic
    // because it would try to access order1 from the orderbook (which was removed)
    assert_eq!(cancel_userref_resp, (vec![order2_uuid.0], vec![]));

    let final_balance = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        final_balance,
        dec!(100_000),
        "Final balance should be $100k after all cancellations"
    );
}
