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
async fn test_stop_loss_buy_triggered(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    // Establish last_traded_price at $50,000 using crossing limit orders
    // User 2 places sell limit at $50k
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
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
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // User 1 places buy limit at $50k (crosses with user 2's sell limit)
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
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
                    .quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
                    .display_quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
                    .volume(dec!(0.01))
                    .build()
                    .unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // Place StopLoss buy with trigger at $52,000
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: OrderTicket::builder(
                        OrderType::StopLoss,
                        OrderSide::Buy,
                        Price {
                            prefix: None,
                            amount: dec!(52000),
                            is_percentage: false,
                        },
                    )
                    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
                    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
                    .volume(dec!(0.1))
                    .build()
                    .unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "StopLoss buy should be placed"
    );

    // Verify USD reserved (0.1 BTC * $52,000 = $5,200)
    let usd_balance = user1.balance(&pg_pool, user1.usd_account_id).await;

    assert_eq!(
        usd_balance,
        dec!(94_300), // 100k - 500 (buy at $50k) - 5200 (stop-loss reserve)
        "USD should be reserved for stop-loss buy, got: {}",
        usd_balance
    );

    // User 2 places sell limit at $53,000 then user 1 market buy to push price above trigger
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: OrderTicket::builder(
                        OrderType::Limit,
                        OrderSide::Sell,
                        Price {
                            prefix: None,
                            amount: dec!(53000),
                            is_percentage: false,
                        },
                    )
                    .quantity(NonZeroDecimal::new(dec!(1.0)).unwrap())
                    .display_quantity(NonZeroDecimal::new(dec!(1.0)).unwrap())
                    .volume(dec!(1.0))
                    .build()
                    .unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // User 1 buys at $53k to trigger the stop-loss (price rises above $52k)
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: OrderTicket::builder(
                        OrderType::Limit,
                        OrderSide::Buy,
                        Price {
                            prefix: None,
                            amount: dec!(53000),
                            is_percentage: false,
                        },
                    )
                    .quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
                    .display_quantity(NonZeroDecimal::new(dec!(0.01)).unwrap())
                    .volume(dec!(0.01))
                    .build()
                    .unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Buy limit should cross and trigger stop-loss, got: {:?}",
        resp
    );

    // Give triggering some time to execute
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify user received BTC from stop-loss execution
    let btc_balance = user1.balance(&pg_pool, user1.btc_account_id).await;

    assert!(
        btc_balance > dec!(9.0),
        "User should have received BTC from stop-loss execution, got: {}",
        btc_balance
    );
}
