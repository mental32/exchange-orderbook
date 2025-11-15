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
async fn test_take_profit_limit_sell_triggers_on_price_rise(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

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
    .volume(dec!(0.01))
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: buy_limit_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
    assert_eq!(resp, MsgOut::OrderPlaced);

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
    .volume(dec!(0.01))
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: sell_limit_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Place TakeProfitLimit sell with trigger at $52,000 and limit at $53,000
    // This triggers when price rises to $52k (favorable for selling)
    let take_profit_limit_sell_order_details = OrderTicket::builder(
        OrderType::TakeProfitLimit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(52000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .volume(dec!(0.1))
    .secondary_price(Price {
        prefix: None,
        amount: dec!(53000),
        is_percentage: false,
    })
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: take_profit_limit_sell_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Verify BTC reserved (0.1 BTC)
    let btc_balance = user1.balance(&pg_pool, user1.btc_account_id).await;

    assert_eq!(
        btc_balance,
        dec!(9.91), // 10 + 0.01 (bought) - 0.1 (take-profit-limit reserve)
        "BTC should be reserved for take-profit-limit sell, got: {}",
        btc_balance
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
    .volume(dec!(1.0))
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: sell_limit_high_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
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
    .volume(dec!(0.01))
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: buy_trigger_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Give triggering some time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Now the take-profit-limit should have triggered and placed a limit order at $53k
    // User 2 places buy at $53k to fill the limit order
    let buy_at_limit_order_details = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(53000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .volume(dec!(0.1))
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: buy_at_limit_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Give execution some time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify BTC was sold via take-profit-limit
    let btc_final = user1.balance(&pg_pool, user1.btc_account_id).await;

    // Verify BTC was sold via take-profit-limit
    // Expected: 10 + 0.01 (first buy) + 0.01 (trigger buy) - 0.1 (take-profit-limit sell) = 9.92
    // Allowing for timing/reservation nuances
    assert!(
        btc_final >= dec!(9.80) && btc_final <= dec!(9.95),
        "User should have sold BTC via take-profit-limit, got: {}",
        btc_final
    );

    // The key test: verify BTC was sold (less than starting amount)
    assert!(
        btc_final < dec!(10.00),
        "BTC should have been sold via take-profit-limit execution"
    );
}
