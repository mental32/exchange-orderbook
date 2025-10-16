use tokio::sync::oneshot;

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
async fn test_ap_actor_cancel_msgout_verification(pg_pool: sqlx::PgPool) {
    let TestFixture {
        user_id,
        btc_usd,
        ap_sender,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    // Place a single order
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_json = serde_json::json!({
        "nonce": 2000,
        "type": "buy",
        "ordertype": "limit",
        "volume": "0.5",
        "displayvol": "0.5",
        "pair": "BTC/USD",
        "price": "45000"
    });

    let place_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd,
        user_id: user_id.clone(),
        order_uuid: order_uuid.clone(),
        order_details: serde_json::from_value(order_json).unwrap(),
    });

    let place_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, place_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(place_resp, Ok(MsgOut::OrderPlaced { .. })));

    // Cancel the order and verify MsgOut structure
    let cancel_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid.clone()),
    };

    let cancel_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    // Destructure and verify the response
    match cancel_resp {
        Ok(MsgOut::OrderCancelled { success, failed }) => {
            // Verify success vec contains exactly the cancelled order UUID
            assert_eq!(success.len(), 1, "Should have exactly 1 successful cancellation");
            assert_eq!(
                success[0], order_uuid,
                "Success vec should contain the cancelled order UUID"
            );

            // Verify failed vec is empty
            assert_eq!(failed.len(), 0, "Should have no failed cancellations");
        }
        Ok(other) => panic!("Expected OrderCancelled, got: {:?}", other),
        Err(e) => panic!("Expected success, got error: {:?}", e),
    }

    // Test error case: cancel non-existent order
    let bogus_uuid = OrderUuid(uuid::Uuid::new_v4());
    let cancel_bogus_msg = MsgIn::CancelOrderBy {
        user_id: user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(bogus_uuid),
    };

    let cancel_bogus_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_bogus_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    // Verify error response
    assert!(
        matches!(cancel_bogus_resp, Err(Error::OrderNotFound)),
        "Cancelling non-existent order should return OrderNotFound, got: {:?}",
        cancel_bogus_resp
    );
}
