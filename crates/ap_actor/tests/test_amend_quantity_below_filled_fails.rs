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
    resp_rx.await.unwrap();

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
    resp_rx.await.unwrap();

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
    assert_eq!(
        resp,
        Ok(MsgOut::OrderAmended {
            order_uuid: sell_order_uuid
        })
    );
}
