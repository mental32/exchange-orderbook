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
async fn test_stop_loss_cancelled_before_trigger(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_btc = user.compute_balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(initial_btc, dec!(10), "Initial BTC should be 10");

    // Place StopLoss sell without triggering
    let stop_loss_order_details = OrderTicket::builder(
        OrderType::StopLoss,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(48000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
    .build()
    .unwrap();

    let stop_loss_uuid = OrderUuid(uuid::Uuid::new_v4());
    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            stop_loss_uuid.clone(),
            stop_loss_order_details,
        )
        .await
        .unwrap();

    // Verify BTC reserved
    let btc_after_place = user.compute_balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_after_place,
        dec!(9.5),
        "BTC should be reserved, got: {}",
        btc_after_place
    );

    // Cancel the stop-loss order
    let resp = proc_router
        .cancel_order(
            CancelOrderBy::TxId(stop_loss_uuid.clone()),
            user.user_id.clone(),
        )
        .await
        .unwrap();
    assert_eq!(resp, (vec![stop_loss_uuid.0], vec![]));

    // Verify refund transaction
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

    // Verify balance restored
    let btc_after_cancel = user.compute_balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_after_cancel,
        dec!(10),
        "BTC balance should be fully restored"
    );
}
