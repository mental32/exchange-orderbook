use ap_actor::proc::MsgError;
use ap_actor::proc::ProcStatus;
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
async fn test_ap_actor_lifecycle(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        proc_router,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Transition to maintenance mode and verify acknowledgement
    let resp = proc_router
        .set_status(btc_usd.clone(), ProcStatus::Maintenance)
        .await;
    assert_eq!(resp.ok(), Some(ProcStatus::Maintenance));

    // Orders should be rejected while the processor is in Maintenance
    let limit_price = Price {
        prefix: None,
        amount: dec!(50_000),
        is_percentage: false,
    };
    let order_while_suspended = OrderTicket::builder(OrderType::Limit, OrderSide::Buy, limit_price)
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .build()
        .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id,
            OrderUuid(uuid::Uuid::new_v4()),
            order_while_suspended,
        )
        .await;
    assert_eq!(resp, Err(MsgError::ProcessorIsSuspended));

    // Bring processor back online
    let resp = proc_router
        .set_status(btc_usd.clone(), ProcStatus::Online)
        .await;
    assert_eq!(resp.ok(), Some(ProcStatus::Online));

    // Orders should now be accepted again
    let order_after_resume = OrderTicket::builder(OrderType::Limit, OrderSide::Buy, limit_price)
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .build()
        .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd,
            user.user_id,
            OrderUuid(uuid::Uuid::new_v4()),
            order_after_resume,
        )
        .await;
    assert!(
        resp.is_ok(),
        "PlaceOrder after returning to Online should succeed"
    );
}
