use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::{AmendOrderArgs, MsgError, MsgIn, MsgOut};
use ap_actor::test::{TestFixture, TestUser, test_ap_actor_fixture};
use matching_engine::decimal::{NonZeroDecimal, dec};
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::{OrderSide, OrderType};
use matching_engine::price::Price;
use tokio::sync::oneshot;

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
    // Expect OrderPlaced or Ok response; don't assert on value here beyond awaiting
    resp_rx.await.unwrap();

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

    assert_eq!(resp, Err(MsgError::InsufficientFunds));
}
