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
async fn test_amend_stop_loss_trigger_price(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        proc_router,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place stop-loss sell: trigger=$48k, qty=0.1
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id,
            order_uuid,
            OrderTicket::builder(
                OrderType::StopLoss,
                OrderSide::Sell,
                Price {
                    prefix: None,
                    amount: dec!(48_000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    // Amend trigger_price to $45k
    let resp = proc_router
        .amend_order(
            order_uuid,
            user.user_id,
            AmendOrderArgs {
                user_id: user.user_id,
                order_uuid,
                new_order_qty: None,
                new_display_qty: None,
                new_limit_price: None,
                new_trigger_price: Some(NonZeroDecimal::new(dec!(45_000)).unwrap()),
                post_only: false,
            },
        )
        .await;

    assert_eq!(resp, Ok(order_uuid));
}
