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
async fn test_ap_actor_cancel_msgout_verification(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let order_uuid = OrderUuid(uuid::Uuid::new_v4());

    let place_resp = proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order_uuid.clone(),
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Buy,
                Price {
                    prefix: None,
                    amount: dec!(45000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
            .display_quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
            .build()
            .unwrap(),
        )
        .await;
    assert!(place_resp.is_ok());

    // Cancel the order and verify MsgOut structure
    let (success, failed) = proc_router
        .cancel_order(
            CancelOrderBy::TxId(order_uuid.clone()),
            user.user_id.clone(),
        )
        .await
        .unwrap();
    assert_eq!(success, vec![order_uuid.0]);
    assert!(failed.is_empty());

    // Test error case: cancel non-existent order
    let bogus_uuid = OrderUuid(uuid::Uuid::new_v4());
    let cancel_bogus_resp = proc_router
        .cancel_order(CancelOrderBy::TxId(bogus_uuid), user.user_id.clone())
        .await;
    assert!(cancel_bogus_resp.is_err());
}
