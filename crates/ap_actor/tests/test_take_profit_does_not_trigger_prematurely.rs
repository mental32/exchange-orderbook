use tokio::sync::oneshot;

use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgIn;
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

#[sqlx::test(migrations = "../../migrations/")]
async fn test_take_profit_does_not_trigger_prematurely(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // User 1 places a limit buy at 50k
    let limit_buy_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .volume(dec!(0.1))
    .build()
    .unwrap();

    let limit_buy = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user1.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: limit_buy_order_details,
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, limit_buy)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Limit buy should be placed"
    );

    // User 2 places a sell TakeProfit at 55k (should NOT trigger yet)
    let take_profit_sell_order_details = OrderTicket::builder(
        OrderType::TakeProfit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(55000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.05)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.05)).unwrap())
    .volume(dec!(0.05))
    .build()
    .unwrap();

    let take_profit_sell = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: take_profit_sell_order_details,
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, take_profit_sell)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Verify BTC is still reserved (not returned)
    let btc_balance_user2 = user2.balance(&pg_pool, user2.btc_account_id).await;

    // Should still have 0.05 BTC reserved
    assert_eq!(
        btc_balance_user2,
        dec!(9.95),
        "User 2 should still have 0.05 BTC reserved (not triggered yet)"
    );

    // User 2 executes a small trade at 50k (below the 55k trigger) - should NOT trigger
    // Note: User 2 already has 0.05 BTC reserved, so only 9.95 BTC available
    let market_sell_order_details = OrderTicket::builder(
        OrderType::Market,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
    .volume(dec!(0.01))
    .build()
    .unwrap();

    let market_sell = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: market_sell_order_details,
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, market_sell)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // BTC should still be reserved (take-profit not triggered)
    let btc_balance_user2_after = user2.balance(&pg_pool, user2.btc_account_id).await;

    // User 2 sold 0.01 BTC, so should have 10 - 0.01 - 0.05 (reserved) = 9.94 BTC
    assert_eq!(
        btc_balance_user2_after,
        dec!(9.94),
        "User 2 should still have 0.05 BTC reserved (take-profit not triggered at 50k)"
    );
}
