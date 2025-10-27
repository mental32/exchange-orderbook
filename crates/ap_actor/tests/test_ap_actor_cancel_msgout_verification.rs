use ap_actor::order_management::CancelOrderBy;
use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::CancelOrderByArgs;
use ap_actor::proc::MsgError;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_cancel_msgout_verification(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user = TestUser::random().create(&pg_pool).await;

    let order_uuid = OrderUuid(uuid::Uuid::new_v4());

    let place_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: order_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(45000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
        .volume(dec!(0.5))
        .build()
        .unwrap(),
    });

    let place_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, place_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(place_resp, Ok(MsgOut::OrderPlaced { .. })));

    // Cancel the order and verify MsgOut structure
    let cancel_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(order_uuid.clone()),
    });

    let cancel_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    // Destructure and verify the response
    match cancel_resp {
        Ok(MsgOut::OrderCancelled { success, failed }) => {
            // Verify success vec contains exactly the cancelled order UUID
            assert_eq!(
                success.len(),
                1,
                "Should have exactly 1 successful cancellation"
            );
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
    let cancel_bogus_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(bogus_uuid),
    });

    let cancel_bogus_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_bogus_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    // Verify error response
    assert!(
        matches!(cancel_bogus_resp, Err(MsgError::OrderNotFound)),
        "Cancelling non-existent order should return OrderNotFound, got: {:?}",
        cancel_bogus_resp
    );
}
