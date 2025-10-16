use tokio::sync::oneshot;

use crate::decimal::dec;
use crate::order_uuid::OrderUuid;
use crate::svc::ap_actor::Error;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::order_management::CancelOrderBy;
use crate::svc::order_management::PlaceOrderArgs;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_gtd_order_expiry(pg_pool: sqlx::PgPool) {
    let TestFixture {
        user_id,
        usd_account_id,
        btc_account_id,
        btc_usd,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd_balance = sqlx::query_scalar!(
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
        initial_usd_balance,
        dec!(100_000),
        "Initial USD balance should be $100k"
    );

    let gtd_buy_json = serde_json::json!({
        "nonce": 123456900,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "50000",
        "timeinforce": "GTD",
        "expiretm": "+2"
    });

    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let gtd_buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_uuid.clone(),
        order_details: serde_json::from_value(gtd_buy_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, gtd_buy_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "GTD buy order should be placed successfully, got: {:?}",
        resp
    );

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
        "User USD balance should be $95k after reserving $5k for GTD order"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let expiry_refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE transaction_type = 'gtd_expiry_refund'
            AND credit_account_id = $1
            "#,
        usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        expiry_refund_count.unwrap_or(0),
        1,
        "Should have exactly 1 gtd_expiry_refund transaction"
    );

    let refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'gtd_expiry_refund'
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
        "Refund amount should be $5,000"
    );

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
        "User USD balance should be fully restored to $100k after expiry"
    );

    let cancel_expired_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, cancel_expired_msg))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(Error::OrderNotFound)),
        "Trying to cancel already-expired order should return OrderNotFound, got: {:?}",
        resp
    );

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

    assert_eq!(initial_btc_balance, dec!(10), "Initial BTC balance should be 10 BTC");

    let gtd_sell_json = serde_json::json!({
        "nonce": 123456901,
        "type": "sell",
        "ordertype": "limit",
        "volume": "0.5",
        "displayvol": "0.5",
        "pair": "BTC/USD",
        "price": "50000",
        "timeinforce": "GTD",
        "expiretm": "+2"
    });

    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let gtd_sell_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: sell_order_uuid.clone(),
        order_details: serde_json::from_value(gtd_sell_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, gtd_sell_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "GTD sell order should be placed successfully, got: {:?}",
        resp
    );

    let btc_balance_after_place = sqlx::query_scalar!(
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
        btc_balance_after_place,
        dec!(9.5),
        "User BTC balance should be 9.5 BTC after reserving 0.5 BTC"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let btc_refund_amount = sqlx::query_scalar!(
        r#"
            SELECT amount
            FROM t_account_tx_journal
            WHERE transaction_type = 'gtd_expiry_refund'
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
        "BTC refund amount should be 0.5 BTC"
    );

    let btc_balance_after_expiry = sqlx::query_scalar!(
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
        btc_balance_after_expiry,
        dec!(10),
        "User BTC balance should be fully restored to 10 BTC after expiry"
    );

    let manual_cancel_json = serde_json::json!({
        "nonce": 123456902,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.2",
        "displayvol": "0.2",
        "pair": "BTC/USD",
        "price": "45000",
        "timeinforce": "GTD",
        "expiretm": "+10"
    });

    let manual_uuid = OrderUuid(uuid::Uuid::new_v4());
    let manual_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: manual_uuid.clone(),
        order_details: serde_json::from_value(manual_cancel_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, manual_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced { .. })),
        "Order with long GTD should be placed successfully"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let cancel_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(manual_uuid),
    };

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderCancelled { .. })),
        "Manual cancel before expiry should succeed"
    );

    let manual_refund_count = sqlx::query_scalar!(
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

    assert!(
        manual_refund_count.unwrap_or(0) > 0,
        "Should have cancel_refund transaction for manual cancellation"
    );

    let order_a_json = serde_json::json!({
        "nonce": 123456903,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.15",
        "displayvol": "0.15",
        "pair": "BTC/USD",
        "price": "49000",
        "timeinforce": "GTD",
        "expiretm": "+2"
    });

    let order_a_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_a = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_a_uuid,
        order_details: serde_json::from_value(order_a_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_a)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced { .. })));

    let order_b_json = serde_json::json!({
        "nonce": 123456904,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.25",
        "displayvol": "0.25",
        "pair": "BTC/USD",
        "price": "47000",
        "timeinforce": "GTD",
        "expiretm": "+5"
    });

    let order_b_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_b = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_b_uuid,
        order_details: serde_json::from_value(order_b_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, order_b)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced { .. })));

    let balance_with_two_orders = sqlx::query_scalar!(
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
        balance_with_two_orders,
        dec!(80_900),
        "Balance should reflect both orders reserved"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let balance_after_first_expiry = sqlx::query_scalar!(
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
        balance_after_first_expiry,
        dec!(88_250),
        "Balance should reflect order A refunded but order B still reserved"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let final_balance_both_expired = sqlx::query_scalar!(
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
        final_balance_both_expired,
        dec!(100_000),
        "Balance should be fully restored after both orders expire"
    );
}
