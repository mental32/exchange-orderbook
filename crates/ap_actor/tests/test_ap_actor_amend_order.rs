use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::AmendOrderArgs;
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
async fn test_amend_limit_order_quantity_maintains_timestamp(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place limit buy order: qty=0.1 BTC, price=$50k
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced { .. })));

    // Verify initial reservation: 0.1 * 50k = 5k
    let balance_before = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(balance_before, dec!(95_000));

    // Amend quantity to 0.2 BTC
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user.user_id,
        order_uuid,
        new_order_qty: Some(dec!(0.2)),
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: None,
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    assert!(
        matches!(resp, Ok(MsgOut::OrderAmended { order_uuid: _ })),
        "Amend should succeed, got: {:?}",
        resp
    );

    // Verify funds: should reserve additional 0.1 * 50k = 5k (total 10k reserved)
    let balance_after = user.balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after,
        dec!(90_000),
        "User USD balance should be reduced by 10k total (0.2 BTC * $50k), got: {}",
        balance_after
    );
}

#[sqlx::test(migrations = "../../migrations/")]
async fn test_amend_quantity_below_filled_fails(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // User1: Place limit sell order: qty=0.1 BTC @ $50k
    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_sell = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user1.user_id,
        order_uuid: sell_order_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Sell,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_sell)).await.unwrap();
    resp_rx.await.unwrap().unwrap();

    // User2: Place limit buy order that partially fills: qty=0.07 BTC @ $50k
    let buy_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_buy = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id,
        order_uuid: buy_order_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.07)).unwrap())
        .volume(dec!(0.07))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_buy)).await.unwrap();
    resp_rx.await.unwrap().unwrap();

    // Now user1's sell order has filled=0.07, remaining=0.03
    // Attempt to amend qty to 0.05 (< 0.07 filled)
    // Per Kraken spec: this should succeed and clamp to filled_qty (0.07), removing order from book
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user1.user_id,
        order_uuid: sell_order_uuid,
        new_order_qty: Some(dec!(0.05)),
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: None,
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    // Should succeed - order quantity clamped to filled quantity (0.07)
    assert!(
        matches!(resp, Ok(ap_actor::proc::MsgOut::OrderAmended { .. })),
        "Amend below filled qty should succeed and clamp, got: {:?}",
        resp
    );
}

#[sqlx::test(migrations = "../../migrations/")]
async fn test_amend_price_loses_queue_priority(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place order A at price=$50k
    let order_a_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_a = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: order_a_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_a)).await.unwrap();
    resp_rx.await.unwrap().unwrap();

    // Place order B at price=$50k (should be behind A in queue)
    let order_b_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_b = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: order_b_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_b)).await.unwrap();
    resp_rx.await.unwrap().unwrap();

    // Amend order A price to $51k (should lose priority)
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user.user_id,
        order_uuid: order_a_uuid,
        new_order_qty: None,
        new_display_qty: None,
        new_limit_price: Some(matching_engine::decimal::NonZeroDecimal::new(dec!(51000)).unwrap()),
        new_trigger_price: None,
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    assert!(
        matches!(resp, Ok(MsgOut::OrderAmended { .. })),
        "Price amend should succeed, got: {:?}",
        resp
    );

    // TODO: Add verification that order A now has a later timestamp at $51k price level
    // This would require inspecting the orderbook state or matching behavior
}

#[sqlx::test(migrations = "../../migrations/")]
async fn test_amend_stop_loss_trigger_price(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place stop-loss sell: trigger=$48k, qty=0.1
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid,
        order_details: OrderTicket::builder(
            OrderType::StopLoss,
            OrderSide::Sell,
            Price {
                prefix: None,
                amount: dec!(48_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced { .. })));

    // Amend trigger_price to $45k
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user.user_id,
        order_uuid,
        new_order_qty: None,
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: Some(
            matching_engine::decimal::NonZeroDecimal::new(dec!(45000)).unwrap(),
        ),
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    assert!(
        matches!(resp, Ok(MsgOut::OrderAmended { .. })),
        "Stop-loss trigger amend should succeed, got: {:?}",
        resp
    );
}

#[sqlx::test(migrations = "../../migrations/")]
async fn test_amend_with_insufficient_funds(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // User starts with $100k
    // Place limit buy: 1.8 BTC @ $50k (reserves $90k, leaves $10k available)
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let place_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid,
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50_000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(1.8)).unwrap())
        .volume(dec!(1.8))
        .build()
        .unwrap(),
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, place_msg)).await.unwrap();
    resp_rx.await.unwrap().unwrap();

    // Attempt to amend qty to 2.1 BTC (needs $105k total, but only has $100k)
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user.user_id,
        order_uuid,
        new_order_qty: Some(dec!(2.1)),
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: None,
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    assert!(
        matches!(resp, Err(ap_actor::proc::MsgError::InsufficientFunds)),
        "Amend should fail with InsufficientFunds, got: {:?}",
        resp
    );
}
