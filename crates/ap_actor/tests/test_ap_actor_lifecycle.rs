use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgError;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::proc::ProcStatus;
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
async fn test_ap_actor_lifecycle(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        ap_sender: actor_tx,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Transition to maintenance mode and verify acknowledgement
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx
            .send((resp_tx, MsgIn::SetStatus(ProcStatus::Maintenance)))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(
        resp.unwrap(),
        MsgOut::StatusChanged(ProcStatus::Maintenance),
        "Setting status to Maintenance should yield StatusChanged(Maintenance)"
    );

    // Orders should be rejected while the processor is in Maintenance
    let limit_price = Price {
        prefix: None,
        amount: dec!(50_000),
        is_percentage: false,
    };
    let order_while_suspended = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id,
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(OrderType::Limit, OrderSide::Buy, limit_price)
            .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .volume(dec!(0.1))
            .build()
            .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx
            .send((resp_tx, order_while_suspended))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Err(MsgError::ProcessorIsSuspended));

    // Bring processor back online
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx
            .send((resp_tx, MsgIn::SetStatus(ProcStatus::Online)))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Ok(MsgOut::StatusChanged(ProcStatus::Online)));

    // Orders should now be accepted again
    let order_after_resume = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user.user_id,
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(OrderType::Limit, OrderSide::Buy, limit_price)
            .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .volume(dec!(0.1))
            .build()
            .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx.send((resp_tx, order_after_resume)).await.unwrap();
        resp_rx.await.unwrap()
    }
    .unwrap();
    assert_eq!(
        resp,
        MsgOut::OrderPlaced,
        "PlaceOrder after returning to Online should succeed"
    );
    assert_eq!(
        resp,
        MsgOut::OrderPlaced,
        "PlaceOrder after returning to Online should succeed"
    );
}
