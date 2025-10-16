use tokio::sync::oneshot;

use crate::asset_code::AssetCode;
use crate::decimal::Decimal;
use crate::decimal::dec;
use crate::order_uuid::OrderUuid;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::order_management::PlaceOrderArgs;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_order_placement(pg_pool: sqlx::PgPool) {
    let TestFixture {
        symbol_vocabulary,
        user_id,
        usd_account_id,
        ap_sender,
        asset_pair_row,
        btc_account_id,
        btc_usd,
        exchange_usd_account_id,
        exchange_btc_account_id,
    } = test_ap_actor_fixture(&pg_pool).await;

    let limit_buy_json = serde_json::json!({
        "nonce": 123456792,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let limit_buy_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
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

    let event_count = sqlx::query_scalar!("SELECT COUNT(*) FROM t_trading_event_source")
        .fetch_one(&pg_pool)
        .await
        .unwrap();
    assert!(
        event_count.unwrap_or(0) > 0,
        "At least one event should be logged in t_trading_event_source"
    );

    let balance = sqlx::query_scalar!(
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
        balance,
        dec!(95_000),
        "User USD balance should be reduced by 5000 (0.1 BTC * $50k), got: {}",
        balance
    );
}
