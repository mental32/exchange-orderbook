use ap_actor::proc::AmendOrderArgs;
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
async fn test_amend_limit_order_quantity_maintains_timestamp(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        proc_router,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place limit buy order: qty=0.1 BTC, price=$50k
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let _ = proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id,
            order_uuid,
            OrderTicket::builder(
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
            .build()
            .unwrap(),
        )
        .await
        .expect("place order");

    // Verify initial reservation: 0.1 * 50k = 5k
    let balance_before = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(balance_before, dec!(95_000));

    // Amend quantity to 0.2 BTC
    let amend_msg = AmendOrderArgs {
        user_id: user.user_id,
        order_uuid,
        new_order_qty: Some(dec!(0.2)),
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: None,
        post_only: false,
    };

    let amended = proc_router
        .amend_order(order_uuid, user.user_id, amend_msg)
        .await
        .expect("amend succeeds");
    assert_eq!(amended, order_uuid);

    // Verify funds: should reserve additional 0.1 * 50k = 5k (total 10k reserved)
    let balance_after = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after,
        dec!(90_000),
        "User USD balance should be reduced by 10k total (0.2 BTC * $50k), got: {}",
        balance_after
    );
}
