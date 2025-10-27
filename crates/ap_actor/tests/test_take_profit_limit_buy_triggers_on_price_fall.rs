use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::dec;
use matching_engine::order_uuid::OrderUuid;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_take_profit_limit_buy_triggers_on_price_fall(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    // Establish last_traded_price at $50,000 using crossing limit orders
    // User 2 places sell limit at $50k
    let sell_limit_json = serde_json::json!({
        "nonce": 120000,
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

    // User 1 places buy limit at $50k (crosses with user 2's sell limit)
    let buy_limit_json = serde_json::json!({
        "nonce": 120001,
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

    // Place TakeProfitLimit buy with trigger at $48,000 and limit at $47,000
    // This triggers when price falls to $48k (favorable for buying)
    let take_profit_limit_buy_json = serde_json::json!({
        "nonce": 120002,
        "type": "buy",
        "ordertype": "take-profit-limit",
        "volume": "0.1",
        "pair": "BTC/USD",
        "price": "48000",        // trigger price
        "price2": "47000",       // limit price
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
                    order_details: serde_json::from_value(take_profit_limit_buy_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "TakeProfitLimit buy should be placed"
    );

    // Verify USD reserved at trigger price (0.1 BTC * $48,000 = $4,800)
    let usd_balance = user1.balance(&pg_pool, user1.usd_account_id).await;

    assert_eq!(
        usd_balance,
        dec!(94_700), // 100k - 500 (buy at $50k) - 4800 (take-profit-limit reserve at trigger price)
        "USD should be reserved at trigger price for take-profit-limit buy, got: {}",
        usd_balance
    );

    // User 1 places buy limit at $46,000
    let buy_limit_low_json = serde_json::json!({
        "nonce": 120003,
        "type": "buy",
        "ordertype": "limit",
        "volume": "1.0",
        "pair": "BTC/USD",
        "price": "46000"
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
                    order_details: serde_json::from_value(buy_limit_low_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(resp, Ok(MsgOut::OrderPlaced)));

    // User 2 sells at $46k to push price below trigger ($48k)
    let sell_trigger_json = serde_json::json!({
        "nonce": 120004,
        "type": "sell",
        "ordertype": "limit",
        "volume": "0.01",
        "pair": "BTC/USD",
        "price": "46000"
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
                    order_details: serde_json::from_value(sell_trigger_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Sell limit should cross and trigger take-profit-limit"
    );

    // Give triggering some time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Now the take-profit-limit should have triggered and placed a limit order at $47k
    // User 2 places sell at $47k to fill the limit order
    let sell_at_limit_json = serde_json::json!({
        "nonce": 120005,
        "type": "sell",
        "ordertype": "limit",
        "volume": "0.1",
        "pair": "BTC/USD",
        "price": "47000"
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
                    order_details: serde_json::from_value(sell_at_limit_json).unwrap(),
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderPlaced)),
        "Sell should fill the triggered limit order at $47k"
    );

    // Give execution some time
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify user received BTC from take-profit-limit execution
    let btc_balance = user1.balance(&pg_pool, user1.btc_account_id).await;

    assert!(
        btc_balance >= dec!(10.10), // Started with 10, bought 0.01 at 50k, bought 0.01 at 46k, bought 0.1 from triggered limit
        "User should have received BTC from take-profit-limit execution at $47k limit price, got: {}",
        btc_balance
    );

    // Verify BTC balance increased (bought via take-profit-limit)
    assert!(
        btc_balance > dec!(10.10),
        "BTC should have been bought via take-profit-limit execution"
    );
}
