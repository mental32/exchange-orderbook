use ap_actor::proc::AmendOrderArgs;
use ap_actor::proc::MsgError;
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
async fn test_amend_with_insufficient_funds(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        proc_router,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // User starts with $100k
    // Place limit buy: 1.8 BTC @ $50k (reserves $90k, leaves $10k available)
    let order_uuid = proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id,
            OrderUuid(uuid::Uuid::new_v4()),
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Buy,
                Price {
                    prefix: None,
                    amount: dec!(50_000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(1.8)).unwrap())
            .build()
            .unwrap(),
        )
        .await
        .expect("order placement should succeed");

    // Attempt to amend qty to 2.1 BTC (needs $105k total, but only has $100k)
    let amend_msg = AmendOrderArgs {
        user_id: user.user_id,
        order_uuid,
        new_order_qty: Some(dec!(2.1)),
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: None,
        post_only: false,
    };

    let resp = proc_router
        .amend_order(order_uuid, user.user_id, amend_msg)
        .await;

    assert_eq!(resp, Err(MsgError::InsufficientFunds));
}
