use tokio::sync::oneshot;

use crate::decimal::dec;
use crate::order_uuid::OrderUuid;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::order_management::CancelOrderBy;
use crate::svc::order_management::PlaceOrderArgs;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_batch_cancel_by_userref(pg_pool: sqlx::PgPool) {
    let TestFixture {
        user_id,
        usd_account_id,
        btc_usd,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Capture initial balance
    let initial_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(initial_balance, dec!(100_000));

    // Place 3 orders all with userref=999
    let order1_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order1_json = serde_json::json!({
        "nonce": 1000,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "50000",
        "userref": 999
    });

    let order1_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order1_uuid.clone(),
        order_details: serde_json::from_value(order1_json).unwrap(),
    });

    let resp1 = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order1_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp1, Ok(MsgOut::OrderPlaced { .. })));

    // Order 2: 0.2 BTC @ $48k = $9,600
    let order2_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order2_json = serde_json::json!({
        "nonce": 1001,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.2",
        "displayvol": "0.2",
        "pair": "BTC/USD",
        "price": "48000",
        "userref": 999
    });

    let order2_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order2_uuid.clone(),
        order_details: serde_json::from_value(order2_json).unwrap(),
    });

    let resp2 = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order2_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp2, Ok(MsgOut::OrderPlaced { .. })));

    // Order 3: 0.15 BTC @ $49k = $7,350
    let order3_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order3_json = serde_json::json!({
        "nonce": 1002,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.15",
        "displayvol": "0.15",
        "pair": "BTC/USD",
        "price": "49000",
        "userref": 999
    });

    let order3_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order3_uuid.clone(),
        order_details: serde_json::from_value(order3_json).unwrap(),
    });

    let resp3 = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order3_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp3, Ok(MsgOut::OrderPlaced { .. })));

    // Total reserved: $5,000 + $9,600 + $7,350 = $21,950
    let balance_after_orders = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        balance_after_orders,
        dec!(78_050),
        "Balance should be $78,050 after reserving $21,950 for 3 orders"
    );

    // Cancel all orders with userref=999 (should cancel all 3)
    let cancel_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::Userref(999),
    };

    let cancel_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    // Verify response structure
    match cancel_resp {
        Ok(MsgOut::OrderCancelled { success, failed }) => {
            assert_eq!(
                success.len(),
                3,
                "Should have cancelled 3 orders, got: {}",
                success.len()
            );

            // Verify all 3 UUIDs are in the success list
            assert!(
                success.contains(&order1_uuid),
                "order1_uuid should be in success list"
            );
            assert!(
                success.contains(&order2_uuid),
                "order2_uuid should be in success list"
            );
            assert!(
                success.contains(&order3_uuid),
                "order3_uuid should be in success list"
            );

            assert_eq!(failed.len(), 0, "Should have no failed cancellations");
        }
        other => panic!("Expected OrderCancelled, got: {:?}", other),
    }

    // Verify all 3 refunds were created
    let refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
            AND credit_account_id = $1
            AND currency = 'USD'
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        refund_count.unwrap_or(0),
        3,
        "Should have exactly 3 cancel_refund transactions"
    );

    // Verify total refund amount equals total reservation
    let total_refund = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(SUM(amount), 0) as "total!"
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
            AND credit_account_id = $1
            AND currency = 'USD'
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        total_refund,
        dec!(21_950),
        "Total refund should be $21,950"
    );

    // Verify balance fully restored
    let final_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        final_balance,
        dec!(100_000),
        "Balance should be fully restored to $100k"
    );
}
