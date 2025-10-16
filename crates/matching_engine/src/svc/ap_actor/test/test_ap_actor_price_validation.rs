use tokio::sync::oneshot;

use crate::asset_code::AssetCode;
use crate::order_uuid::OrderUuid;
use crate::svc::ap_actor::Error;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::order_management::PlaceOrderArgs;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_price_validation(pg_pool: sqlx::PgPool) {
    let TestFixture {
        symbol_vocabulary: vocab,
        user_id,
        ap_sender,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let relative_price_json = serde_json::json!({
        "nonce": 123456789,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "+100"
    });

    let relative_price_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: (
            AssetCode::from_str_and_vocabulary("BTC", &vocab).unwrap(),
            AssetCode::from_str_and_vocabulary("USD", &vocab).unwrap(),
        ),
        user_id: user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: serde_json::from_value(relative_price_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, relative_price_order))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(Error::NoReferencePrice)),
        "Relative price with no last_traded_price should return NoReferencePrice, got: {:?}",
        resp
    );

    let zero_price_json = serde_json::json!({
        "nonce": 123456790,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "0"
    });

    let zero_price_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: serde_json::from_value(zero_price_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, zero_price_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(Error::InvalidPrice)),
        "Zero price should return InvalidPrice, got: {:?}",
        resp
    );

    let validate_only_json = serde_json::json!({
        "nonce": 123456791,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "50000",
        "validate": true
    });

    let validate_only_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: serde_json::from_value(validate_only_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, validate_only_order))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::OrderValidated)),
        "Validate-only order should return ValidatedOrder, got: {:?}",
        resp
    );
}
