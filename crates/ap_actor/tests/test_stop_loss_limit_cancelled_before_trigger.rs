use ap_actor::proc::CancelOrderByArgs;
use tokio::sync::oneshot;

use ap_actor::order_management::CancelOrderBy;
use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::dec;
use matching_engine::order_uuid::OrderUuid;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_stop_loss_limit_cancelled_before_trigger(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user = TestUser::random().create(&pg_pool).await;

    let initial_usd = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(initial_usd, dec!(100_000), "Initial USD should be $100k");

    // Place StopLossLimit buy order
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let stop_loss_limit_buy_json = serde_json::json!({
        "nonce": 150001,
        "type": "buy",
        "ordertype": "stop-loss-limit",
        "volume": "0.1",
        "pair": "BTC/USD",
        "price": "52000",      // trigger price
        "price2": "53000",     // limit price
        "trigger": "Last"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user.user_id.clone(),
                    order_uuid,
                    order_details: serde_json::from_value(stop_loss_limit_buy_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "StopLossLimit buy should be placed"
    );

    // Verify USD reserved at trigger price (0.1 BTC * $52,000 = $5,200)
    let usd_after_place = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_place,
        dec!(94_800),
        "USD should be reserved at trigger price (0.1 * $52k = $5,200)"
    );

    // Cancel the order before it triggers
    let cancel_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order should be cancelled successfully, got: {:?}",
        resp
    );

    // Give cancellation some time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify funds were refunded
    let usd_after_cancel = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_cancel,
        dec!(100_000),
        "USD balance should be fully restored after cancellation, got: {}",
        usd_after_cancel
    );
}
