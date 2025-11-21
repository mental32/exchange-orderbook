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
async fn test_ap_actor_partial_fill_cancel(pg_pool: sqlx::PgPool) {
    let user1 = TestUser::random().create(&pg_pool).await;
    let user2 = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd = user2.compute_balance(&pg_pool, user2.usd_account_id).await;
    let initial_btc = user1.compute_balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(initial_usd, dec!(100_000));
    assert_eq!(initial_btc, dec!(10));

    // Step 1: Place a sell order (maker) for 1.0 BTC @ $50,000
    let maker_order_uuid = OrderUuid(uuid::Uuid::new_v4());

    proc_router
        .place_order(
            btc_usd.clone(),
            user1.user_id.clone(),
            maker_order_uuid.clone(),
            OrderTicket::builder(
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
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    let btc_after_maker = user1.compute_balance(&pg_pool, user1.btc_account_id).await;
    assert_eq!(
        btc_after_maker,
        dec!(9.0),
        "Should have 9 BTC remaining after reserving 1.0 BTC"
    );

    // Step 2: Place a buy order (taker) for 0.6 BTC @ $50,000 (will partially fill)
    // This will match 0.6 BTC, leaving 0.4 BTC resting on the sell side
    let taker_order_uuid = OrderUuid(uuid::Uuid::new_v4());

    proc_router
        .place_order(
            btc_usd.clone(),
            user2.user_id.clone(),
            taker_order_uuid.clone(),
            OrderTicket::builder(
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
            .build()
            .unwrap(),
        )
        .await
        .unwrap();

    let btc_after_fill = user2.compute_balance(&pg_pool, user2.btc_account_id).await;
    let usd_after_fill = user2.compute_balance(&pg_pool, user2.usd_account_id).await;
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
    let cancel_resp = proc_router
        .cancel_order(
            CancelOrderBy::TxId(maker_order_uuid.clone()),
            user1.user_id.clone(),
        )
        .await
        .unwrap();

    assert_eq!(cancel_resp, (vec![maker_order_uuid.0], vec![]));

    // Step 4: Verify refund is ONLY for the unfilled portion (0.4 BTC)
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

    let final_btc = user1.compute_balance(&pg_pool, user1.btc_account_id).await;
    let final_usd = user2.compute_balance(&pg_pool, user2.usd_account_id).await;
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
