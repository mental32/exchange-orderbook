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
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    // Amend trigger_price to $45k
    let amend_msg = MsgIn::AmendOrder(AmendOrderArgs {
        user_id: user.user_id,
        order_uuid,
        new_order_qty: None,
        new_display_qty: None,
        new_limit_price: None,
        new_trigger_price: Some(NonZeroDecimal::new(dec!(45_000)).unwrap()),
        post_only: false,
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    ap_sender.send((resp_tx, amend_msg)).await.unwrap();
    let resp = resp_rx.await.unwrap();

    assert_eq!(resp, Ok(MsgOut::OrderAmended { order_uuid }));
}
