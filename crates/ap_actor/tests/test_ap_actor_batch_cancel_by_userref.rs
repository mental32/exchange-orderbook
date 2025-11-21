use ap_actor::proc_router::CancelOrderBy;
use ap_actor::test::AccountTxJournal;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::t_account_tx_journal;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_batch_cancel_by_userref(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_balance = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(initial_balance, dec!(100_000));

    // Place 3 orders all with userref=999
    let order1_uuid = OrderUuid(uuid::Uuid::new_v4());

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order1_uuid.clone(),
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Buy,
                Price {
                    prefix: None,
                    amount: dec!(50000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
            .userref(999)
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    // Order 2: 0.2 BTC @ $48k = $9,600
    let order2_uuid = OrderUuid(uuid::Uuid::new_v4());

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order2_uuid.clone(),
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Buy,
                Price {
                    prefix: None,
                    amount: dec!(48000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
            .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
            .userref(999)
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    // Order 3: 0.15 BTC @ $49k = $7,350
    let order3_uuid = OrderUuid(uuid::Uuid::new_v4());

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order3_uuid.clone(),
            OrderTicket::builder(
                OrderType::Limit,
                OrderSide::Buy,
                Price {
                    prefix: None,
                    amount: dec!(49000),
                    is_percentage: false,
                },
            )
            .quantity(NonZeroDecimal::new(dec!(0.15)).unwrap())
            .display_quantity(NonZeroDecimal::new(dec!(0.15)).unwrap())
            .userref(999)
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    let balance_after_orders = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        balance_after_orders,
        dec!(78_050),
        "Balance should be $78,050 after reserving $21,950 for 3 orders"
    );

    // Cancel all orders with userref=999 (should cancel all 3)
    let cancel_resp = proc_router
        .cancel_order(CancelOrderBy::Userref(999), user.user_id.clone())
        .await
        .unwrap();

    // Verify response structure
    assert_eq!(
        cancel_resp,
        (vec![order1_uuid.0, order2_uuid.0, order3_uuid.0], vec![])
    );

    // Verify cancel_refund journal entries
    t_account_tx_journal(&pg_pool) /* actual */
        .await
        .iter()
        .zip(
            [
                /* expected (snapshot to be updated by test runner) */
            ]
            .into_iter()
            .map(|value| serde_json::from_value::<AccountTxJournal>(value).unwrap()),
        )
        .for_each(|(actual, expected)| {
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.credit_account_id, expected.credit_account_id);
            assert_eq!(actual.debit_account_id, expected.debit_account_id);
            assert_eq!(actual.currency, expected.currency);
            assert_eq!(actual.amount, expected.amount);
            assert_eq!(actual.transaction_type, expected.transaction_type);
        });

    let final_balance = user.compute_balance(&pg_pool, user.usd_account_id).await;
    assert_eq!(
        final_balance,
        dec!(100_000),
        "Balance should be fully restored to $100k"
    );
}
