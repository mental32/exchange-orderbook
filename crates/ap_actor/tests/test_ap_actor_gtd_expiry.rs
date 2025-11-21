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
use matching_engine::orderbook::TimeInForce;
use matching_engine::price::Price;
use std::time;
use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::sleep_until;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_gtd_order_expiry(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd_balance = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        initial_usd_balance,
        dec!(100_000),
        "Initial USD balance should be $100k"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;
    let order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let gtd_buy_order = OrderTicket::builder(
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
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order_uuid.clone(),
            gtd_buy_order.clone(),
        )
        .await
        .unwrap();

    let balance_after_place = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        balance_after_place,
        dec!(95_000),
        "User USD balance should be $95k after reserving $5k for GTD order"
    );

    sleep_until(Instant::now() + Duration::from_millis(2000)).await;

    let final_balance = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        final_balance,
        dec!(100_000),
        "User USD balance should be fully restored to $100k after expiry"
    );

    let resp = proc_router
        .cancel_order(CancelOrderBy::TxId(order_uuid), user.user_id.clone())
        .await
        .unwrap();
    assert_eq!(
        resp,
        (
            vec![],
            vec![(order_uuid.0, "Failed to cancel order".to_owned())]
        )
    );

    let initial_btc_balance = user.compute_balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        initial_btc_balance,
        dec!(10),
        "Initial BTC balance should be 10 BTC"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;
    let sell_order_uuid = OrderUuid(uuid::Uuid::new_v4());
    let gtd_sell_order = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Sell,
        Price {
            prefix: None,
            amount: dec!(50000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.5)).unwrap())
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            sell_order_uuid.clone(),
            gtd_sell_order.clone(),
        )
        .await
        .unwrap();

    let btc_balance_after_place = user.compute_balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_balance_after_place,
        dec!(9.5),
        "User BTC balance should be 9.5 BTC after reserving 0.5 BTC"
    );

    sleep_until(Instant::now() + Duration::from_millis(2010)).await;

    let btc_balance_after_expiry = user.compute_balance(&pg_pool, user.btc_account_id).await;

    assert_eq!(
        btc_balance_after_expiry,
        dec!(10),
        "User BTC balance should be fully restored to 10 BTC after expiry"
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;
    let manual_uuid = OrderUuid(uuid::Uuid::new_v4());
    let manual_order = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(45000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.2)).unwrap())
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            manual_uuid.clone(),
            manual_order,
        )
        .await
        .unwrap();

    sleep_until(Instant::now() + Duration::from_millis(2000)).await;

    let resp = proc_router
        .cancel_order(
            CancelOrderBy::TxId(manual_uuid.clone()),
            user.user_id.clone(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp,
        (
            vec![],
            vec![(manual_uuid.0, "Failed to cancel order".to_owned())]
        )
    );

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;
    let order_a_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_a = OrderTicket::builder(
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
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order_a_uuid.clone(),
            order_a,
        )
        .await
        .unwrap();

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;
    let order_b_uuid = OrderUuid(uuid::Uuid::new_v4());
    let order_b = OrderTicket::builder(
        OrderType::Limit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(47000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.25)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.25)).unwrap())
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            order_b_uuid.clone(),
            order_b,
        )
        .await
        .unwrap();

    sleep_until(Instant::now() + Duration::from_millis(100)).await;

    let balance_with_two_orders = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        balance_with_two_orders,
        dec!(80_900),
        "Balance should reflect both orders reserved"
    );

    sleep_until(Instant::now() + Duration::from_millis(2000)).await;

    let final_balance_both_expired = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        final_balance_both_expired,
        dec!(100_000),
        "Balance should be fully restored after both orders expire"
    );

    // Snapshot journal assertions
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
}
