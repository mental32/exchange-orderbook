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
use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::sleep_until;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_stop_loss_gtd_expiry(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(initial_usd, dec!(100_000), "Initial USD should be $100k");

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;

    // Place StopLoss buy with GTD expiring in 2 seconds
    let stop_loss_gtd_order_details = OrderTicket::builder(
        OrderType::StopLoss,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(52000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    let OrderUuid(_u0) = proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            stop_loss_gtd_order_details,
        )
        .await
        .unwrap();

    // Verify USD reserved
    let usd_after_place = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_place,
        dec!(94_800),
        "USD should be reserved (0.1 * $52k = $5,200)"
    );

    // Wait until expiry should have processed.
    sleep_until(Instant::now() + Duration::from_millis(2000)).await;

    // Verify balance restored
    let usd_after_expiry = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_expiry,
        dec!(100_000),
        "USD balance should be fully restored after expiry"
    );
}
