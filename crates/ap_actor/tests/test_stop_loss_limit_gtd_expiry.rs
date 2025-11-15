use matching_engine::orderbook::TimeInForce;
use tokio::sync::oneshot;

use ap_actor::order_management::PlaceOrderArgs;
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
use std::time;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_stop_loss_limit_gtd_expiry(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        btc_usd, ap_sender, ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let initial_usd = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(initial_usd, dec!(100_000), "Initial USD should be $100k");

    let expiry = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 2;

    // Place StopLossLimit buy with GTD expiring in 2 seconds
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
    .volume(dec!(0.1))
    .secondary_price(Price {
        prefix: None,
        amount: dec!(53000),
        is_percentage: false,
    })
    .time_in_force(TimeInForce::GoodTilDate(expiry as u64))
    .build()
    .unwrap();

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((
                resp_tx,
                MsgIn::PlaceOrder(PlaceOrderArgs {
                    base_quote: btc_usd.clone(),
                    user_id: user.user_id.clone(),
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    order_details: stop_loss_limit_gtd_order_details,
                }),
            ))
            .await
            .unwrap();
        resp_rx.await.unwrap().unwrap()
    };
    assert_eq!(resp, MsgOut::OrderPlaced);

    // Verify USD reserved at trigger price (0.1 BTC * $52,000 = $5,200)
    let usd_after_place = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_place,
        dec!(94_800),
        "USD should be reserved at trigger price (0.1 * $52k = $5,200)"
    );

    // Wait for expiry (without triggering the order)
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify expiry refund exists (could be stop_loss_gtd_expiry_refund or stop_loss_limit_gtd_expiry_refund)
    let refund_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM t_account_tx_journal
            WHERE (transaction_type = 'stop_loss_gtd_expiry_refund'
                   OR transaction_type = 'stop_loss_limit_gtd_expiry_refund')
            AND credit_account_id = $1
            "#,
        user.usd_account_id
    )
    .fetch_one(&pg_pool)
    .await
    .unwrap();

    assert_eq!(
        refund_count.unwrap_or(0),
        1,
        "Should have GTD expiry refund transaction"
    );

    // Verify balance restored
    let usd_after_expiry = user.balance(&pg_pool, user.usd_account_id).await;

    assert_eq!(
        usd_after_expiry,
        dec!(100_000),
        "USD balance should be fully restored after expiry"
    );
}
