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
async fn test_take_profit_limit_does_not_trigger_prematurely(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    // Establish last_traded_price at $50,000 using crossing limit orders
    // User 1 places buy limit at $50k
    let buy_limit_json = serde_json::json!({
        "nonce": 160000,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.01",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(buy_limit_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // User 2 places sell limit at $50k (crosses with user 1's buy limit)
    let sell_limit_json = serde_json::json!({
        "nonce": 160001,
        "type": "sell",
        "ordertype": "limit",
        "volume": "0.01",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(sell_limit_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // Place TakeProfitLimit sell with trigger at $55,000 and limit at $56,000
    let take_profit_limit_sell_json = serde_json::json!({
        "nonce": 160002,
        "type": "sell",
        "ordertype": "take-profit-limit",
        "volume": "0.1",
        "pair": "BTC/USD",
        "price": "55000",        // trigger price
        "price2": "56000",       // limit price
        "trigger": "Last"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(take_profit_limit_sell_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "TakeProfitLimit sell should be placed"
    );

    // Verify BTC reserved (0.1 BTC)
    let btc_after_place = user1.balance(&pg_pool, user1.btc_account_id).await;

    assert_eq!(
        btc_after_place,
        dec!(9.91), // 10 + 0.01 (bought) - 0.1 (take-profit-limit reserve)
        "BTC should be reserved for take-profit-limit sell, got: {}",
        btc_after_place
    );

    // User 2 places sell limit at $54,000 (BELOW trigger price of $55k)
    let sell_below_trigger_json = serde_json::json!({
        "nonce": 160003,
        "type": "sell",
        "ordertype": "limit",
        "volume": "1.0",
        "pair": "BTC/USD",
        "price": "54000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(sell_below_trigger_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // User 1 buys at $54k (still below $55k trigger)
    let buy_below_trigger_json = serde_json::json!({
        "nonce": 160004,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.01",
        "pair": "BTC/USD",
        "price": "54000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(buy_below_trigger_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Buy at $54k should execute but NOT trigger the take-profit-limit"
    );

    // Give some time for any potential triggering
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify BTC is still reserved (take-profit-limit should NOT have triggered)
    let btc_after_below_trigger = user1.balance(&pg_pool, user1.btc_account_id).await;

    // Should be 9.91 + 0.01 = 9.92, with 0.1 still reserved
    assert_eq!(
        btc_after_below_trigger,
        dec!(9.92),
        "BTC should still be reserved, take-profit-limit should NOT have triggered yet, got: {}",
        btc_after_below_trigger
    );

    // NOW push price to $55k to actually trigger
    // User 2 places sell at $55k
    let sell_at_trigger_json = serde_json::json!({
        "nonce": 160005,
        "type": "sell",
        "ordertype": "limit",
        "volume": "1.0",
        "pair": "BTC/USD",
        "price": "55000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(sell_at_trigger_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // User 1 buys at $55k to trigger
    let buy_at_trigger_json = serde_json::json!({
        "nonce": 160006,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.01",
        "pair": "BTC/USD",
        "price": "55000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user1.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(buy_at_trigger_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // Give triggering some time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // User 2 buys at $56k to fill the triggered limit order
    let buy_at_limit_json = serde_json::json!({
        "nonce": 160007,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "pair": "BTC/USD",
        "price": "56000"
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user2.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: serde_json::from_value(buy_at_limit_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // Give execution some time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify take-profit-limit executed (BTC sold)
    let btc_final = user1.balance(&pg_pool, user1.btc_account_id).await;

    // 10 + 0.01 (first) + 0.01 (below trigger) + 0.01 (at trigger) - 0.1 (take-profit-limit sold) = 9.93
    assert!(
        btc_final >= dec!(9.90) && btc_final <= dec!(9.95),
        "Take-profit-limit should have triggered and executed at $55k, got: {}",
        btc_final
    );
}
