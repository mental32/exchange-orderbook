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
async fn test_stop_loss_sell_triggered(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Step 1: Establish last_traded_price at $50,000 using crossing limit orders
    // User 1 places sell limit at $50k
    let sell_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(50000),
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
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // User 2 places buy limit at $50k (crosses with user 1's sell limit)
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
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            buy_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Step 2: Place StopLoss sell order with trigger at $48,000
    let stop_loss_sell_order_details = OrderTicket::builder(
        OrderType::StopLoss,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(48000),
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
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            stop_loss_sell_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Step 3: Verify BTC reserved (balance drops from 10 to 9.9)
    let btc_balance = user1.compute_balance(&pg_pool, user1.btc_account_id).await;

    assert_eq!(
        btc_balance,
        dec!(8.9), // 10 - 0.01 (sold in crossing) - 0.99 (remaining sell limit) - 0.1 (stop-loss)
        "BTC should be reserved for stop-loss sell order"
    );

    // Step 4: Place market sell to drop price below trigger ($48,000)
    // First user 2 places buy limit at $47,500
    let buy_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(47500),
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
            buy_limit_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Now user 1 sells at $47.5k to trigger the stop-loss (price drops below $48k)
    let sell_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(47500),
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
            sell_trigger_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Give async trigger check time to execute
    sleep_until(Instant::now() + Duration::from_millis(100)).await;

    // Step 5: Verify stop-loss executed (user received USD from sale)
    let usd_balance = user1.compute_balance(&pg_pool, user1.usd_account_id).await;

    // User should have received USD from the stop-loss market sell
    // Initial: 100k, spent on buy limit: -47.5k, received from triggered stop-loss sell: +4.75k (0.1 BTC * $47,500)
    assert!(
        usd_balance > dec!(52_000),
        "User should have received USD from stop-loss execution, got: {}",
        usd_balance
    );
}
