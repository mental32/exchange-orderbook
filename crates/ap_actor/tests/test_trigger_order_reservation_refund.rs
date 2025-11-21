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

/// Regression: ensure trigger orders reserve once and refund fully on cancel after trigger
#[sqlx::test(migrations = "../../migrations/")]
async fn test_trigger_order_reservation_refund(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Establish last_traded_price at $50,000 using crossing limit orders
    let buy_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for buy limit order quantity"),
    )
    .display_quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for buy limit order display quantity"),
    )
    .build()
    .expect("Failed to build buy limit order for establishing last traded price");

    proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            buy_limit_order_details,
        )
        .await
        .expect("Failed to place buy limit order to establish last traded price");

    let sell_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for sell limit order quantity"),
    )
    .display_quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for sell limit order display quantity"),
    )
    .build()
    .expect("Failed to build sell limit order for establishing last traded price");

    proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_limit_order_details,
        )
        .await
        .expect("Failed to place sell limit order to establish last traded price");

    // Place take-profit-limit sell that will be triggered later
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let take_profit_limit_sell_order_details =
        OrderTicket::builder(
            OrderType::TakeProfitLimit,
            OrderSide::Sell,
            Price {
                prefix: None,
                amount: dec!(52000),
                is_percentage: false,
            },
        )
        .quantity(
            NonZeroDecimal::new(dec!(0.1))
                .expect("Failed to create non-zero decimal for take-profit-limit order quantity"),
        )
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).expect(
            "Failed to create non-zero decimal for take-profit-limit order display quantity",
        ))
        .secondary_price(Price {
            prefix: None,
            amount: dec!(53000),
            is_percentage: false,
        })
        .build()
        .expect("Failed to build take-profit-limit sell order");

    proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            order_uuid,
            take_profit_limit_sell_order_details,
        )
        .await
        .expect("Failed to place take-profit-limit sell order");

    // BTC reserve should be 0.1
    let btc_after_place = user1.compute_balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(
        btc_after_place,
        dec!(9.91),
        "reserve should debit 0.1 BTC at placement"
    );

    // Push last trade price to $52,000 to trigger the order
    let sell_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(52000),
            is_percentage: false,
        },
    )
    .quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for trigger sell order quantity"),
    )
    .display_quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for trigger sell order display quantity"),
    )
    .build()
    .expect("Failed to build sell order to trigger take-profit-limit");

    proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            sell_trigger_order_details,
        )
        .await
        .expect("Failed to place sell order to trigger take-profit-limit");

    let buy_trigger_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(52000),
            is_percentage: false,
        },
    )
    .quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for trigger buy order quantity"),
    )
    .display_quantity(
        NonZeroDecimal::new(dec!(0.01))
            .expect("Failed to create non-zero decimal for trigger buy order display quantity"),
    )
    .build()
    .expect("Failed to build buy order to trigger take-profit-limit");

    proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            buy_trigger_order_details,
        )
        .await
        .expect("Failed to place buy order to trigger take-profit-limit");

    // Allow trigger processing
    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // After trigger, reservation should not increase (still 0.1 BTC held)
    let btc_after_trigger = user1.compute_balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(
        btc_after_trigger,
        dec!(9.92),
        "trigger should not double-reserve base asset"
    );

    // Cancel the triggered (resting) order and ensure refund
    let resp = proc_router
        .cancel_order(CancelOrderBy::TxId(order_uuid), user1.user_id.clone())
        .await
        .expect("Failed to cancel triggered order");
    assert_eq!(resp, (vec![order_uuid.0], vec![]));

    sleep_until(Instant::now() + Duration::from_millis(200)).await;

    // Reservation should be fully released, leaving only the two small buys
    let btc_after_cancel = user1.compute_balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(
        btc_after_cancel,
        dec!(10.02),
        "refund should restore balance to starting + executed buys"
    );
}
