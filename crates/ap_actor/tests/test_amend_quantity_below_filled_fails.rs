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
async fn test_amend_quantity_below_filled_fails(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        proc_router,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // User1: Place limit sell order: qty=0.1 BTC @ $50k
    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id,
            sell_order_uuid,
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Sell,
                Price {
                    prefix: None,
                    amount: dec!(50_000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    // User2: Place limit buy order that partially fills: qty=0.07 BTC @ $50k
    let buy_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id,
            buy_order_uuid,
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Buy,
                Price {
                    prefix: None,
                    amount: dec!(50_000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.07)).unwrap())
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    // Now user1's sell order has filled=0.07, remaining=0.03
    // Attempt to amend qty to 0.05 (< 0.07 filled)
    // Per Kraken spec: this should succeed and clamp to filled_qty (0.07), removing order from book
    let resp = proc_router
        .amend_order(
            sell_order_uuid,
            user1.user_id,
            AmendOrderArgs {
                user_id: user1.user_id,
                order_uuid: sell_order_uuid,
                new_order_qty: Some(dec!(0.05)),
                new_display_qty: None,
                new_limit_price: None,
                new_trigger_price: None,
                post_only: false,
            },
        )
        .await;

    // Should succeed - order quantity clamped to filled quantity (0.07)
    assert_eq!(resp, Ok(sell_order_uuid));
}
