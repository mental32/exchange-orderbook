use ap_actor::proc::MsgOut;
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
async fn test_stop_loss_limit_buy_triggered(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Establish last_traded_price at $50,000 using crossing limit orders
    // User 2 places sell limit at $50k
    let sell_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // User 1 places buy limit at $50k (crosses with user 2's sell limit)
    let buy_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            buy_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Place StopLossLimit buy with trigger at $52,000 and limit at $53,000
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

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            stop_loss_limit_buy_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Verify USD reserved at trigger price (0.1 BTC * $52,000 = $5,200)
    let usd_balance = user1.compute_balance(&pg_pool, user1.usd_account_id).await;

    assert_eq!(
        usd_balance,
        dec!(94_300), // 100k - 500 (buy at $50k) - 5200 (stop-loss-limit reserve at trigger price)
        "USD should be reserved at trigger price for stop-loss-limit buy, got: {}",
        usd_balance
    );

    // User 2 places sell limit at $54,000
    let sell_limit_high_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(54000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(1.0)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(1.0)).unwrap())
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_limit_high_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // User 1 buys at $54k to push price above trigger ($52k)
    let buy_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(54000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            buy_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Give triggering some time to execute
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // Now the stop-loss-limit should have triggered and placed a limit order at $53k
    // User 2 places sell at $53k to fill the limit order
    let sell_at_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(53000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_at_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Give execution some time
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // Verify user received BTC from stop-loss-limit execution
    let btc_balance = user1.compute_balance(&pg_pool, user1.btc_account_id).await;

    assert!(
        btc_balance >= dec!(10.09), // Started with 10, bought 0.01 at 50k, bought 0.1 from triggered limit
        "User should have received BTC from stop-loss-limit execution at $53k limit price, got: {}",
        btc_balance
    );

    // Verify USD spent at limit price ($53k for 0.1 BTC = $5,300)
    let usd_final = user1.compute_balance(&pg_pool, user1.usd_account_id).await;

    assert!(
        usd_final <= dec!(94_160), // 100k - 500 (first buy) - 540 (trigger buy) - 5300 (limit execution)
        "User should have spent USD at limit price $53k, got: {}",
        usd_final
    );
}
