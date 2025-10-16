use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::asset_code::AssetCode;
use crate::order_uuid::OrderUuid;
use crate::orderbook::Orderbook;
use crate::svc::ap_actor::ApState;
use crate::svc::ap_actor::Error;
use crate::svc::ap_actor::MsgIn;
use crate::svc::ap_actor::MsgOut;
use crate::svc::ap_actor::asset_processor_loop;
use crate::svc::ap_actor::test::TestFixture;
use crate::svc::ap_actor::test::test_ap_actor_fixture;
use crate::svc::order_management::PlaceOrderArgs;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_lifecycle(pg_pool: sqlx::PgPool) {
    let TestFixture {
        symbol_vocabulary,
        asset_pair_row,
        user_id,
        ap_sender: actor_tx,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx.send((resp_tx, MsgIn::Suspend)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::WillSuspend)),
        "Suspend should return WillSuspend"
    );

    let order_json = serde_json::json!({
        "nonce": 123456789,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.1",
        "displayvol": "0.1",
        "pair": "BTC/USD",
        "price": "50000"
    });

    let order_while_suspended = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: serde_json::from_value(order_json).unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx
            .send((resp_tx, order_while_suspended))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Err(Error::ProcessorIsSuspended)),
        "PlaceOrder while suspended should return ProcessorIsSuspended error"
    );

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx.send((resp_tx, MsgIn::Resume)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::WillResume)),
        "Resume should return WillResume, got: {:?}",
        resp
    );

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        actor_tx.send((resp_tx, MsgIn::Shutdown)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::WillShutdown)),
        "Shutdown should return WillShutdown"
    );

    let (ap_snd, ap_recv) = mpsc::channel(16);
    let proc = ApState {
        pg_pool: pg_pool.clone(),
        orderbook: Orderbook::new_empty(),
        tracking: Default::default(),
        asset_pair_row: asset_pair_row.clone(),
        symbol_vocabulary: symbol_vocabulary.clone(),
        expiry_queue: vec![],
    };
    tokio::spawn(asset_processor_loop(ap_recv, proc));

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_snd.send((resp_tx, MsgIn::Suspend)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(
        matches!(resp, Ok(MsgOut::WillSuspend)),
        "New actor after shutdown should work"
    );
}
