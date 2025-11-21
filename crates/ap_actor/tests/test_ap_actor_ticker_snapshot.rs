use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_ticker_snapshot_updates_after_trade(pg_pool: sqlx::PgPool) {
    let seller = TestUser::random().create(&pg_pool).await;
    let buyer = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        proc_router,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let OrderUuid(_u0) = proc_router
        .place_order(
            btc_usd.clone(),
            seller.user_id,
            OrderUuid(uuid::Uuid::new_v4()),
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Sell,
                Price {
                    prefix: None,
                    amount: dec!(50_000),
                    is_percentage: false,
                },
            )
            .quantity(dec!(0.5).into())
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    let OrderUuid(_u1) = proc_router
        .place_order(
            btc_usd.clone(),
            buyer.user_id,
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
            .quantity(dec!(0.5).into())
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    let snapshot = proc_router
        .ticker_snapshot(btc_usd.clone())
        .await
        .expect("ticker snapshot");

    let expected_volume = dec!(0.5);
    assert_eq!(
        snapshot.last_trade_price,
        Some(dec!(50_000)),
        "last trade price should reflect matched order"
    );
    assert_eq!(
        snapshot.last_trade_volume,
        Some(expected_volume),
        "last trade volume should match filled amount"
    );
    assert_eq!(
        snapshot.volume_today, expected_volume,
        "today volume should equal the single trade volume"
    );
    assert_eq!(
        snapshot.volume_24h, expected_volume,
        "24h volume should equal the single trade volume"
    );
    assert_eq!(
        snapshot.trades_today, 1,
        "should have counted one trade for today"
    );
    assert_eq!(
        snapshot.trades_24h, 1,
        "should have counted one trade over the rolling window"
    );
}
