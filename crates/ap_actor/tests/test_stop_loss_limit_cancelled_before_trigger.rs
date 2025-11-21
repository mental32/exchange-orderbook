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
use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::sleep_until;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_stop_loss_limit_cancelled_before_trigger(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(initial_usd, dec!(100_000), "Initial USD should be $100k");

    // Place StopLossLimit buy order
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let stop_loss_limit_buy_order_details = OrderTicket::builder(
        OrderType::StopLossLimit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(52000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .secondary_price(Price {
        prefix: None,
        amount: dec!(53000),
        is_percentage: false,
    })
    .build()
    .unwrap();

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order_uuid,
            stop_loss_limit_buy_order_details,
        )
        .await
        .unwrap();

    // Verify USD reserved at trigger price (0.1 BTC * $52,000 = $5,200)
    let usd_after_place = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_place,
        dec!(94_800),
        "USD should be reserved at trigger price (0.1 * $52k = $5,200)"
    );

    // Cancel the order before it triggers
    let resp = proc_router
        .cancel_order(CancelOrderBy::TxId(order_uuid), user.user_id.clone())
        .await
        .unwrap();
    assert_eq!(resp, (vec![order_uuid.0], vec![]));

    // Give cancellation some time to process
    sleep_until(Instant::now() + Duration::from_millis(100)).await;

    // Verify funds were refunded
    let usd_after_cancel = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_cancel,
        dec!(100_000),
        "USD balance should be fully restored after cancellation, got: {}",
        usd_after_cancel
    );
}
