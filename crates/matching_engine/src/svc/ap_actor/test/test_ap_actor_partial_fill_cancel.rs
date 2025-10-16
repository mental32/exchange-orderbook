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
async fn test_ap_actor_partial_fill_cancel(pg_pool: sqlx::PgPool) {
    let TestFixture {
        user_id,
        usd_account_id,
        btc_account_id,
        btc_usd,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Capture initial balances
    let initial_usd = sqlx::query_scalar!(
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

    let initial_btc = sqlx::query_scalar!(
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

    assert_eq!(initial_usd, dec!(100_000));
    assert_eq!(initial_btc, dec!(10));

    // Step 1: Place a sell order (maker) for 1.0 BTC @ $50,000
    let maker_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let maker_order_json = serde_json::json!({
        "nonce": 3000,
        "type": "sell",
        "ordertype": "limit",
        "volume": "1.0",
        "displayvol": "1.0",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let maker_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: maker_order_uuid.clone(),
        order_details: serde_json::from_value(maker_order_json).unwrap(),
    });

    let maker_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, maker_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(maker_resp, Ok(MsgOut::OrderPlaced { .. })));

    // Verify 1.0 BTC was reserved
    let btc_after_maker = sqlx::query_scalar!(
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
        btc_after_maker,
        dec!(9.0),
        "Should have 9 BTC remaining after reserving 1.0 BTC"
    );

    // Step 2: Place a buy order (taker) for 0.6 BTC @ $50,000 (will partially fill)
    // This will match 0.6 BTC, leaving 0.4 BTC resting on the sell side
    let taker_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let taker_order_json = serde_json::json!({
        "nonce": 3001,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.6",
        "displayvol": "0.6",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let taker_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: taker_order_uuid.clone(),
        order_details: serde_json::from_value(taker_order_json).unwrap(),
    });

    let taker_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, taker_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(taker_resp, Ok(MsgOut::OrderPlaced { .. })));

    // Verify partial fill occurred:
    // - User should now have 9.6 BTC (9.0 + 0.6 from fill)
    // - User should have $70,000 USD (100k - 30k spent on 0.6 BTC)
    let btc_after_fill = sqlx::query_scalar!(
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

    let usd_after_fill = sqlx::query_scalar!(
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
        btc_after_fill,
        dec!(9.6),
        "Should have 9.6 BTC after partial fill (9.0 + 0.6)"
    );
    assert_eq!(
        usd_after_fill,
        dec!(70_000),
        "Should have $70k USD after spending $30k on 0.6 BTC"
    );

    // Step 3: Cancel the maker order (which still has 0.4 BTC resting)
    let cancel_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(maker_order_uuid.clone()),
    };

    let cancel_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    match cancel_resp {
        Ok(MsgOut::OrderCancelled { success, failed }) => {
            assert_eq!(success.len(), 1, "Should cancel 1 order");
            assert_eq!(success[0], maker_order_uuid, "Should cancel the maker order");
            assert_eq!(failed.len(), 0, "Should have no failures");
        }
        other => panic!("Expected OrderCancelled, got: {:?}", other),
    }

    // Step 4: Verify refund is ONLY for the unfilled portion (0.4 BTC)
    let btc_refund = sqlx::query_scalar!(
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
        btc_refund,
        dec!(0.4),
        "Refund should be 0.4 BTC (unfilled portion), not the original 1.0 BTC"
    );

    // Step 5: Verify final balance is correct
    // Started with 10 BTC
    // Sold 0.6 BTC (now have 9.4 BTC + $30k USD gained)
    // Final: 10 BTC total
    let final_btc = sqlx::query_scalar!(
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

    let final_usd = sqlx::query_scalar!(
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

    // User should have:
    // BTC: 10 (initial) - 0.6 (sold and settled) = 9.4 BTC
    assert_eq!(
        final_btc,
        dec!(10.0),
        "Final BTC should be 10.0 (9.6 after fill + 0.4 refund)"
    );

    // USD: 100k (initial) - 30k (bought 0.6 BTC) + 30k (sold 0.6 BTC) = 100k
    assert_eq!(
        final_usd,
        dec!(100_000),
        "Final USD should be $100k (net zero from self-trade)"
    );
}
