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
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_take_profit_sell_triggers_on_price_rise(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // User 1 places a limit buy at 50k

    let limit_buy = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user1.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .volume(dec!(0.2))
        .build()
        .unwrap(),
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

    // User 2 places a limit sell at 50k to establish the market price

    let limit_sell = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
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
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, limit_sell)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Limit sell should be placed and matched, establishing price at 50k"
    );

    // User 2 places a sell TakeProfit at 55k (should trigger when price rises to 55k)

    let take_profit_sell = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
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
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, take_profit_sell)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "TakeProfit sell should be queued"
    );

    // Verify BTC was reserved (0.05 BTC)
    let btc_balance_user2 = user2.balance(&pg_pool, user2.btc_account_id).await;

    // User 2 sold 0.01 BTC and reserved 0.05 BTC, so: 10 - 0.01 - 0.05 = 9.94 BTC remaining
    assert_eq!(
        btc_balance_user2,
        dec!(9.94),
        "User 2 should have 0.05 BTC reserved for take-profit sell"
    );

    // User 1 places a limit buy at 55k (resting on book)

    let trigger_buy = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user1.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(55000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
        .volume(dec!(0.2))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, trigger_buy)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Trigger buy should be placed"
    );

    // User 2 places a limit sell at 55k to actually execute a trade and set price to 55k
    // This should trigger the take-profit

    let trigger_sell = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
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
        .volume(dec!(0.01))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, trigger_sell)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Trigger sell should execute at 55k and trigger take-profit, got: {:?}",
        resp
    );

    // Verify the take-profit was triggered by checking user 2's USD balance increased
    let usd_balance_user2 = user2.balance(&pg_pool, user2.usd_account_id).await;

    // User 2: sold 0.01 BTC at 50k = +500 USD
    //         sold 0.01 BTC at 55k = +550 USD
    //         take-profit sold 0.05 BTC at 55k = +2750 USD
    // Starting with 100,000, should now have 100,000 + 500 + 550 + 2750 = 103,800
    assert_eq!(
        usd_balance_user2,
        dec!(103_800),
        "User 2 should have received USD from both sells and triggered take-profit, got: {}",
        usd_balance_user2
    );
}
