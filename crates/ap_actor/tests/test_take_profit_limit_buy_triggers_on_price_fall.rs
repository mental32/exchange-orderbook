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
async fn test_take_profit_limit_buy_triggers_on_price_fall(pg_pool: sqlx::PgPool) {
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

    // Place TakeProfitLimit buy with trigger at $48,000 and limit at $47,000
    // This triggers when price falls to $48k (favorable for buying)
    let take_profit_limit_buy_order_details = OrderTicket::builder(
        OrderType::TakeProfitLimit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(48000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .secondary_price(Price {
        prefix: None,
        amount: dec!(47000),
        is_percentage: false,
    })
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            take_profit_limit_buy_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Verify USD reserved at trigger price (0.1 BTC * $48,000 = $4,800)
    let usd_balance = user1.compute_balance(&pg_pool, user1.usd_account_id).await;

    assert_eq!(
        usd_balance,
        dec!(94_700), // 100k - 500 (buy at $50k) - 4800 (take-profit-limit reserve at trigger price)
        "USD should be reserved at trigger price for take-profit-limit buy, got: {}",
        usd_balance
    );

    // User 1 places buy limit at $46,000
    let buy_limit_low_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(46000),
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
            buy_limit_low_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced)
        .unwrap();
    assert_eq!(resp, MsgOut::OrderPlaced);

    // User 2 sells at $46k to push price below trigger ($48k)
    let sell_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(46000),
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
            sell_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Sell limit should cross and trigger take-profit-limit"
    );

    // Give triggering some time to execute
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // Now the take-profit-limit should have triggered and placed a limit order at $47k
    // User 2 places sell at $47k to fill the limit order
    let sell_at_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(47000),
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

    // Verify user received BTC from take-profit-limit execution
    let btc_balance = user1.compute_balance(&pg_pool, user1.btc_account_id).await;

    assert!(
        btc_balance >= dec!(10.10), // Started with 10, bought 0.01 at 50k, bought 0.01 at 46k, bought 0.1 from triggered limit
        "User should have received BTC from take-profit-limit execution at $47k limit price, got: {}",
        btc_balance
    );

    // Verify BTC balance increased (bought via take-profit-limit)
    assert!(
        btc_balance > dec!(10.10),
        "BTC should have been bought via take-profit-limit execution"
    );
}
