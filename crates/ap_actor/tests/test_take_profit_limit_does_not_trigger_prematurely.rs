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
async fn test_take_profit_limit_does_not_trigger_prematurely(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Establish last_traded_price at $50,000 using crossing limit orders
    // User 1 places buy limit at $50k
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
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // User 2 places sell limit at $50k (crosses with user 1's buy limit)
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
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Place TakeProfitLimit sell with trigger at $55,000 and limit at $56,000
    let take_profit_limit_sell_order_details = OrderTicket::builder(
        OrderType::TakeProfitLimit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(55000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .secondary_price(Price {
        prefix: None,
        amount: dec!(56000),
        is_percentage: false,
    })
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            take_profit_limit_sell_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(
        resp,
        Ok(MsgOut::OrderPlaced),
        "TakeProfitLimit sell should be placed"
    );

    // Verify BTC reserved (0.1 BTC)
    let btc_after_place = user1.compute_balance(&pg_pool, user1.btc_account_id).await;

    assert_eq!(
        btc_after_place,
        dec!(9.91), // 10 + 0.01 (bought) - 0.1 (take-profit-limit reserve)
        "BTC should be reserved for take-profit-limit sell, got: {}",
        btc_after_place
    );

    // User 2 places sell limit at $54,000 (BELOW trigger price of $55k)
    let sell_below_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
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
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_below_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // User 1 buys at $54k (still below $55k trigger)
    let buy_below_trigger_order_details = OrderTicket::builder(
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
            buy_below_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(
        resp,
        Ok(MsgOut::OrderPlaced),
        "Buy at $54k should execute but NOT trigger the take-profit-limit"
    );

    // Give some time for any potential triggering
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // Verify BTC is still reserved (take-profit-limit should NOT have triggered)
    let btc_after_below_trigger = user1.compute_balance(&pg_pool, user1.btc_account_id).await;

    // Should be 9.91 + 0.01 = 9.92, with 0.1 still reserved
    assert_eq!(
        btc_after_below_trigger,
        dec!(9.92),
        "BTC should still be reserved, take-profit-limit should NOT have triggered yet, got: {}",
        btc_after_below_trigger
    );

    // NOW push price to $55k to actually trigger
    // User 2 places sell at $55k
    let sell_at_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(55000),
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
            sell_at_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // User 1 buys at $55k to trigger
    let buy_at_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(55000),
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
            buy_at_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Give triggering some time
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // User 2 buys at $56k to fill the triggered limit order
    // User 1 buys at $54k (still below $55k trigger)
    let buy_limit_order_details = OrderTicket::builder(
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
            buy_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Give execution some time
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // Verify take-profit-limit executed (BTC sold)
    let btc_final = user1.compute_balance(&pg_pool, user1.btc_account_id).await;

    // 10 + 0.01 (first) + 0.01 (below trigger) + 0.01 (at trigger) - 0.1 (take-profit-limit sold) = 9.93
    assert!(
        btc_final >= dec!(9.90) && btc_final <= dec!(9.95),
        "Take-profit-limit should have triggered and executed at $55k, got: {}",
        btc_final
    );
}
