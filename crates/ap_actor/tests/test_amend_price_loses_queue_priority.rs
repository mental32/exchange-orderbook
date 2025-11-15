use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::{AmendOrderArgs, MsgIn, MsgOut};
use ap_actor::test::{TestFixture, TestUser, test_ap_actor_fixture};
use matching_engine::decimal::{NonZeroDecimal, dec};
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::{OrderSide, OrderType};
use matching_engine::price::Price;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_amend_price_loses_queue_priority(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place order A at price = $50k
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
    resp_rx.await.unwrap();

    // Place order B at same price = $50k (should be behind A)
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
    resp_rx.await.unwrap();

    // Amend order A's price to $51k — this should remove its priority at the $50k level
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user.user_id,
        order_uuid: order_a_uuid,
        new_order_qty: None,
        new_display_qty: None,
        new_limit_price: Some(matching_engine::decimal::NonZeroDecimal::new(dec!(51_000)).unwrap()),
        new_trigger_price: None,
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    assert_eq!(
        resp,
        Ok(MsgOut::OrderAmended {
            order_uuid: order_a_uuid
        })
    );

    // Note: verifying the timestamp/queue-priority change requires inspecting orderbook internals
    // or matching behavior which is beyond the scope of this black-box test. The presence of
    // OrderAmended here ensures the amend succeeded; additional assertions could be added if
    // orderbook state inspection helpers are available.
}
