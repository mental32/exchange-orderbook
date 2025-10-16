use tokio::sync::oneshot;

use crate::asset_code::AssetCode;
use crate::decimal::Decimal;
use crate::decimal::dec;
use crate::order_uuid::OrderUuid;
use crate::svc::ap_actor::Error;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::order_management::CancelOrderBy;
use crate::svc::order_management::PlaceOrderArgs;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_order_cancellation(pg_pool: sqlx::PgPool) {
    let TestFixture {
        symbol_vocabulary,
        user_id,
        usd_account_id,
        btc_account_id,
        btc_usd,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Capture initial USD balance before placing order
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

    assert_eq!(
        initial_balance,
        dec!(100_000),
        "Initial USD balance should be $100k, got: {}",
        initial_balance
    );

    // Place limit buy order: 0.1 BTC @ $50,000 = $5,000 reserved
    let limit_buy_json = serde_json::json!({
        "nonce": 123456800,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let limit_buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_uuid.clone(),
        order_details: serde_json::from_value(limit_buy_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, limit_buy_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Limit buy order should be placed successfully, got: {:?}",
        resp
    );

    // Verify funds were reserved (balance should decrease by $5,000)
    let balance_after_place = sqlx::query_scalar!(
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
        balance_after_place,
        dec!(95_000),
        "User USD balance should be reduced by $5k (0.1 BTC * $50k), got: {}",
        balance_after_place
    );

    // Cancel the order by TxId
    let cancel_order_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid.clone()),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_order_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order should be cancelled successfully, got: {:?}",
        resp
    );

    // Verify refund transaction was created
    let refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
            AND credit_account_id = $1
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        refund_count.unwrap_or(0),
        1,
        "Should have exactly 1 cancel_refund transaction"
    );

    // Verify refund amount is correct ($5,000)
    let refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
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
        refund_amount,
        dec!(5_000),
        "Refund amount should be $5,000 (0.1 BTC * $50k), got: {}",
        refund_amount
    );

    // Verify balance is fully restored to initial amount
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
        "User USD balance should be fully restored to $100k after cancellation, got: {}",
        final_balance
    );

    // Test error case: try to cancel the same order again
    let cancel_again_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_again_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(Error::OrderNotFound)),
        "Cancelling already-cancelled order should return OrderNotFound, got: {:?}",
        resp
    );

    // Section 2: Sell order cancellation (BTC refund)
    let initial_btc_balance = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        initial_btc_balance,
        dec!(10),
        "Initial BTC balance should be 10 BTC, got: {}",
        initial_btc_balance
    );

    // Place limit sell order: 0.5 BTC @ $50,000 (reserves 0.5 BTC)
    let limit_sell_json = serde_json::json!({
        "nonce": 123456801,
        "type": "sell",
        "ordertype": "limit",
        "volume": "0.5",
        "displayvol": "0.5",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let limit_sell_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: sell_order_uuid.clone(),
        order_details: serde_json::from_value(limit_sell_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, limit_sell_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Limit sell order should be placed successfully, got: {:?}",
        resp
    );

    // Verify BTC was reserved (balance should decrease by 0.5 BTC)
    let btc_balance_after_sell = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        btc_balance_after_sell,
        dec!(9.5),
        "User BTC balance should be reduced by 0.5 BTC, got: {}",
        btc_balance_after_sell
    );

    // Cancel the sell order
    let cancel_sell_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(sell_order_uuid),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_sell_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Sell order should be cancelled successfully, got: {:?}",
        resp
    );

    // Verify BTC refund transaction
    let btc_refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'cancel_refund'
            AND credit_account_id = $1
            AND currency = 'BTC'
            "#,
        btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        btc_refund_amount,
        dec!(0.5),
        "BTC refund amount should be 0.5 BTC, got: {}",
        btc_refund_amount
    );

    // Verify BTC balance fully restored
    let btc_balance_after_cancel = sqlx::query_scalar!(
        r#"
            SELECT COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE credit_account_id = $1),
                0
            ) - COALESCE(
                (SELECT SUM(amount) FROM t_account_tx_journal WHERE debit_account_id = $1),
                0
            ) as "balance!"
            "#,
        btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        btc_balance_after_cancel,
        dec!(10),
        "User BTC balance should be fully restored to 10 BTC after cancellation, got: {}",
        btc_balance_after_cancel
    );

    // Section 3: Cancel by ClientOrderId
    let limit_buy_with_cl_ord_id_json = serde_json::json!({
        "nonce": 123456802,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.2",
        "displayvol": "0.2",
        "pair": "BTC/USD",
        "price": "45000",
        "cl_ord_id": "test-order-cl-123"
    });

    let cl_ord_id_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_with_cl_ord_id = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: cl_ord_id_uuid,
        order_details: serde_json::from_value(limit_buy_with_cl_ord_id_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, order_with_cl_ord_id))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order with cl_ord_id should be placed successfully, got: {:?}",
        resp
    );

    // Verify USD reserved (0.2 BTC * $45k = $9,000)
    let usd_balance_before_cl_cancel = sqlx::query_scalar!(
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
        usd_balance_before_cl_cancel,
        dec!(91_000),
        "User USD balance should be $91k after reserving $9k (0.2 BTC * $45k), got: {}",
        usd_balance_before_cl_cancel
    );

    // Cancel by ClientOrderId
    let cancel_by_cl_ord_id_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::ClientOrderId("test-order-cl-123".to_string()),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_by_cl_ord_id_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Cancel by ClientOrderId should succeed, got: {:?}",
        resp
    );

    // Verify USD refund ($9,000)
    let usd_balance_after_cl_cancel = sqlx::query_scalar!(
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
        usd_balance_after_cl_cancel,
        dec!(100_000),
        "User USD balance should be restored to $100k after cancel by cl_ord_id, got: {}",
        usd_balance_after_cl_cancel
    );

    // Section 4: Cancel by Userref
    let limit_buy_with_userref_json = serde_json::json!({
        "nonce": 123456803,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.3",
        "displayvol": "0.3",
        "pair": "BTC/USD",
        "price": "48000",
        "userref": 42
    });

    let userref_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_with_userref = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: userref_uuid,
        order_details: serde_json::from_value(limit_buy_with_userref_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_with_userref)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order with userref should be placed successfully, got: {:?}",
        resp
    );

    // Verify USD reserved (0.3 BTC * $48k = $14,400)
    let usd_balance_before_userref_cancel = sqlx::query_scalar!(
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
        usd_balance_before_userref_cancel,
        dec!(85_600),
        "User USD balance should be $85,600 after reserving $14,400 (0.3 BTC * $48k), got: {}",
        usd_balance_before_userref_cancel
    );

    // Cancel by Userref
    let cancel_by_userref_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::Userref(42),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_by_userref_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Cancel by Userref should succeed, got: {:?}",
        resp
    );

    // Verify USD refund ($14,400)
    let usd_balance_after_userref_cancel = sqlx::query_scalar!(
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
        usd_balance_after_userref_cancel,
        dec!(100_000),
        "User USD balance should be restored to $100k after cancel by userref, got: {}",
        usd_balance_after_userref_cancel
    );

    // Section 5: NoOpenPositions error
    use common_core::web::middleware::clerk::ClerkUserId;

    let user_no_orders_id = "user-test-noorders";
    sqlx::query!(
        "INSERT INTO t_user_data (id, name, email, password_hash) VALUES ($1, $2, $3, $4)",
        user_no_orders_id,
        "User With No Orders",
        "noorders@example.com",
        &[] as &[u8]
    )
    .execute(&pg_pool)
    .await
    .unwrap();

    let user_no_orders = ClerkUserId(user_no_orders_id.to_string());

    // Try to cancel order for user with no positions
    let cancel_no_positions_msg = MsgIn::CancelOrderBy {
        user_id: user_no_orders,
        cancel_order_by: CancelOrderBy::TxId(OrderUuid(uuid::Uuid::new_v4())),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_no_positions_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(Error::NoOpenPositions)),
        "Cancelling order for user with no positions should return NoOpenPositions, got: {:?}",
        resp
    );

    // Section 6: Multiple orders - cancel specific one
    // Place first order with userref=100
    let order_a_json = serde_json::json!({
        "nonce": 123456804,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.15",
        "displayvol": "0.15",
        "pair": "BTC/USD",
        "price": "49000",
        "userref": 100
    });

    let order_a_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_a = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_a_uuid.clone(),
        order_details: serde_json::from_value(order_a_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_a)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order A should be placed successfully, got: {:?}",
        resp
    );

    // Place second order with userref=200
    let order_b_json = serde_json::json!({
        "nonce": 123456805,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.25",
        "displayvol": "0.25",
        "pair": "BTC/USD",
        "price": "47000",
        "userref": 200
    });

    let order_b_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_b = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_b_uuid.clone(),
        order_details: serde_json::from_value(order_b_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_b)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order B should be placed successfully, got: {:?}",
        resp
    );

    // Verify both orders reserved funds (0.15 * $49k + 0.25 * $47k = $7,350 + $11,750 = $19,100)
    let usd_balance_with_two_orders = sqlx::query_scalar!(
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
        usd_balance_with_two_orders,
        dec!(80_900),
        "User USD balance should be $80,900 after reserving $19,100 for two orders, got: {}",
        usd_balance_with_two_orders
    );

    // Cancel only order A (userref=100)
    let cancel_order_a_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::Userref(100),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_order_a_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order A should be cancelled successfully, got: {:?}",
        resp
    );

    // Verify only order A was refunded ($7,350), order B still active
    let usd_balance_after_cancel_a = sqlx::query_scalar!(
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
        usd_balance_after_cancel_a,
        dec!(88_250),
        "User USD balance should be $88,250 after cancelling order A ($7,350 refund), got: {}",
        usd_balance_after_cancel_a
    );

    // Cancel order B (userref=200)
    let cancel_order_b_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::Userref(200),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_order_b_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Order B should be cancelled successfully, got: {:?}",
        resp
    );

    // Verify both orders fully refunded
    let usd_balance_final = sqlx::query_scalar!(
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
        usd_balance_final,
        dec!(100_000),
        "User USD balance should be fully restored to $100k after cancelling both orders, got: {}",
        usd_balance_final
    );
}
