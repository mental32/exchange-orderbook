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
async fn test_ap_actor_order_placement(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender, btc_usd, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let limit_buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
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

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, limit_buy_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    let event_count = sqlx::query_scalar!("SELECT COUNT(*) FROM t_trading_event_source")
        .fetch_one(&pg_pool)
        .await
        .unwrap();
    assert!(
        event_count.unwrap_or(0) > 0,
        "At least one event should be logged in t_trading_event_source"
    );

    let balance = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        balance,
        dec!(95_000),
        "User USD balance should be reduced by 5000 (0.1 BTC * $50k), got: {}",
        balance
    );
}
