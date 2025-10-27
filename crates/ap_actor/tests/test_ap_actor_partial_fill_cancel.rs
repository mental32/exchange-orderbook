use ap_actor::order_management::CancelOrderBy;
use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::CancelOrderByArgs;
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
async fn test_ap_actor_partial_fill_cancel(pg_pool: sqlx::PgPool) {
    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let initial_usd = user2.balance(&pg_pool, user2.usd_account_id).await;
    let initial_btc = user1.balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(initial_usd, dec!(100_000));
    assert_eq!(initial_btc, dec!(10));

    // Step 1: Place a sell order (maker) for 1.0 BTC @ $50,000
    let maker_order_uuid = OrderUuid(uuid::Uuid::new_v4());

    let maker_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user1.user_id.clone(),
        order_uuid: maker_order_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Sell,
            Price {
                prefix: None,
                amount: dec!(50000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(1.0)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(1.0)).unwrap())
        .volume(dec!(1.0))
        .build()
        .unwrap(),
    });

    let maker_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, maker_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(maker_resp, Ok(MsgOut::OrderPlaced { .. })));

    let btc_after_maker = user1.balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(
        btc_after_maker,
        dec!(9.0),
        "Should have 9 BTC remaining after reserving 1.0 BTC"
    );

    // Step 2: Place a buy order (taker) for 0.6 BTC @ $50,000 (will partially fill)
    // This will match 0.6 BTC, leaving 0.4 BTC resting on the sell side
    let taker_order_uuid = OrderUuid(uuid::Uuid::new_v4());

    let taker_msg = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user2.user_id.clone(),
        order_uuid: taker_order_uuid.clone(),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(50000),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.6)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.6)).unwrap())
        .volume(dec!(0.6))
        .build()
        .unwrap(),
    });

    let taker_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, taker_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert!(matches!(taker_resp, Ok(MsgOut::OrderPlaced { .. })));

    let btc_after_fill = user2.balance(&pg_pool, user2.btc_account_id).await;
    let usd_after_fill = user2.balance(&pg_pool, user2.usd_account_id).await;
    assert_eq!(
        btc_after_fill,
        dec!(10.6),
        "User2 should have 10.6 BTC after partial fill (10.0 + 0.6)"
    );
    assert_eq!(
        usd_after_fill,
        dec!(70_000),
        "User2 should have $70k USD after spending $30k on 0.6 BTC"
    );

    // Step 3: Cancel the maker order (which still has 0.4 BTC resting)
    let cancel_msg = MsgIn::CancelOrderBy(CancelOrderByArgs {
        user_id: user1.user_id.clone(),
        cancel_order_by: CancelOrderBy::TxId(maker_order_uuid.clone()),
    });

    let cancel_resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, cancel_msg)).await.unwrap();
        resp_rx.await.unwrap()
    };

    match cancel_resp {
        Ok(MsgOut::OrderCancelled { success, failed }) => {
            assert_eq!(success.len(), 1, "Should cancel 1 order");
            assert_eq!(
                success[0], maker_order_uuid,
                "Should cancel the maker order"
            );
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
        user1.btc_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        btc_refund,
        dec!(0.4),
        "Refund should be 0.4 BTC (unfilled portion), not the original 1.0 BTC"
    );

    let final_btc = user1.balance(&pg_pool, user1.btc_account_id).await;
    let final_usd = user2.balance(&pg_pool, user2.usd_account_id).await;
    assert_eq!(
        final_btc,
        dec!(9.4),
        "User1 (seller) final BTC should be 9.4 (10.0 - 0.6 sold)"
    );
    assert_eq!(
        final_usd,
        dec!(70_000),
        "User2 (buyer) final USD should be $70k (100k - 30k spent)"
    );
}
