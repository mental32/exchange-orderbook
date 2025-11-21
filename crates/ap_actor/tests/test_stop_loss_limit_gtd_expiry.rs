use ap_actor::proc::MsgOut;
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
async fn test_stop_loss_limit_gtd_expiry(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd,
        proc_router,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(initial_usd, dec!(100_000), "Initial USD should be $100k");

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;

    let stop_loss_limit_gtd_order_details = OrderTicket::builder(
        OrderType::StopLossLimit,
        OrderSide::Buy,
        Price {
            prefix: None,
            amount: dec!(52000),
            is_percentage: false,
        },
    )
    .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
    .secondary_price(Price {
        prefix: None,
        amount: dec!(53000),
        is_percentage: false,
    })
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    let resp = proc_router
        .place_order(
            btc_usd.clone(),
            user.user_id.clone(),
            OrderUuid(uuid::Uuid::new_v4()),
            stop_loss_limit_gtd_order_details,
        )
        .await
        .map(|_| MsgOut::OrderPlaced);
    assert_eq!(resp, Ok(MsgOut::OrderPlaced));

    let usd_after_place = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_place,
        dec!(94_800),
        "USD should be reserved at trigger price (0.1 * $52k = $5,200)"
    );

    // Wait for expiry (without triggering the order)
    sleep_until(Instant::now() + Duration::from_millis(2000)).await;

    // Verify balance restored
    let usd_after_expiry = user.compute_balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_expiry,
        dec!(100_000),
        "USD balance should be fully restored after expiry"
    );

    // Verify expiry refund exists
    t_account_tx_journal(&pg_pool) /* actual */
        .await
        .iter()
        .zip(
            [
                /* expected (this is a snapshot value, so it may update) */
                serde_json::json!({
                    "id": 1,
                    "credit_account_id": 3,
                    "debit_account_id": 1,
                    "currency": "USD",
                    "amount": 100000.0,
                    "transaction_type": "test_deposit",
                }),
                serde_json::json!({
                    "id": 2,
                    "credit_account_id": 4,
                    "debit_account_id": 2,
                    "currency": "BTC",
                    "amount": 10.0,
                    "transaction_type": "test_deposit",
                }),
                serde_json::json!({
                    "id": 3,
                    "credit_account_id": 1,
                    "debit_account_id": 3,
                    "currency": "USD",
                    "amount": 5200.0,
                    "transaction_type": "reserve asset",
                }),
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
