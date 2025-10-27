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
async fn test_stop_loss_cancelled_before_trigger(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user = TestUser::random().create(&pg_pool).await;

    let initial_btc = user.balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(initial_btc, dec!(10), "Initial BTC should be 10");

    // Place StopLoss sell without triggering
    let stop_loss_json = serde_json::json!({
        "nonce": 300001,
        "type": "sell",
        "ordertype": "stop-loss",
        "volume": "0.5",
        "pair": "BTC/USD",
        "price": "48000",
        "trigger": "Last"
    });

    let stop_loss_uuid = OrderUuid(uuid::Uuid::new_v4());
    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user.user_id.clone(),
                    order_uuid: stop_loss_uuid.clone(),
                    order_details: serde_json::from_value(stop_loss_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // Verify BTC reserved
    let btc_after_place = user.balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_after_place,
        dec!(9.5),
        "BTC should be reserved, got: {}",
        btc_after_place
    );

    // Cancel the stop-loss order
    let cancel_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(stop_loss_uuid),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Should successfully cancel stop-loss order, got: {:?}",
        resp
    );

    // Verify refund transaction exists
    let refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'stop_loss_cancel_refund'
            AND credit_account_id = $1
            "#,
        user.btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        refund_count.unwrap_or(0),
        1,
        "Should have stop_loss_cancel_refund transaction"
    );

    // Verify balance restored
    let btc_after_cancel = user.balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_after_cancel,
        dec!(10),
        "BTC balance should be fully restored"
    );
}
