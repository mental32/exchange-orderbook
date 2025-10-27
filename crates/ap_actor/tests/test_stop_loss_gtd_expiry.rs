use tokio::sync::oneshot;

use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::dec;
use matching_engine::order_uuid::OrderUuid;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_stop_loss_gtd_expiry(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user = TestUser::random().create(&pg_pool).await;

    let initial_usd = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(initial_usd, dec!(100_000), "Initial USD should be $100k");

    // Place StopLoss buy with GTD expiring in 2 seconds
    let stop_loss_gtd_json = serde_json::json!({
        "nonce": 400001,
        "type": "buy",
        "ordertype": "stop-loss",
        "volume": "0.1",
        "pair": "BTC/USD",
        "price": "52000",
        "trigger": "Last",
        "timeinforce": "GTD",
        "expiretm": "+2"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(stop_loss_gtd_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // Verify USD reserved
    let usd_after_place = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_place,
        dec!(94_800),
        "USD should be reserved (0.1 * $52k = $5,200)"
    );

    // Wait for expiry
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify expiry refund exists
    let refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'stop_loss_gtd_expiry_refund'
            AND credit_account_id = $1
            "#,
        user.usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        refund_count.unwrap_or(0),
        1,
        "Should have stop_loss_gtd_expiry_refund transaction"
    );

    // Verify balance restored
    let usd_after_expiry = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_expiry,
        dec!(100_000),
        "USD balance should be fully restored after expiry"
    );
}
