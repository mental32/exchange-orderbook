use ap_actor::order_management::PlaceOrderArgs;
use ap_actor::proc::MsgError;
use ap_actor::proc::MsgIn;
use ap_actor::proc::MsgOut;
use ap_actor::test::TestFixture;
use ap_actor::test::TestUser;
use ap_actor::test::test_ap_actor_fixture;
use matching_engine::asset_code::AssetCode;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::decimal::dec;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;
use std::str::FromStr;
use tokio::sync::oneshot;

#[sqlx::test(migrations = "../../migrations/")]
async fn test_ap_actor_price_validation(pg_pool: sqlx::PgPool) {
    let user = TestUser::random().create(&pg_pool).await;

    let TestFixture {
        symbol_vocabulary: vocab,
        ap_sender,
        btc_usd,
        ..
    } = test_ap_actor_fixture(&pg_pool).await;

    let relative_price_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: (
            AssetCode::from_str_and_vocabulary("BTC", &vocab).unwrap(),
            AssetCode::from_str_and_vocabulary("USD", &vocab).unwrap(),
        ),
        user_id: user.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(100),
                is_percentage: true,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, relative_price_order))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Err(MsgError::NoReferencePrice));

    let zero_price_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
            OrderType::Limit,
            OrderSide::Buy,
            Price {
                prefix: None,
                amount: dec!(0),
                is_percentage: false,
            },
        )
        .quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .display_quantity(NonZeroDecimal::new(dec!(0.1)).unwrap())
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender.send((resp_tx, zero_price_order)).await.unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Err(MsgError::InvalidPrice));

    let validate_only_order = MsgIn::PlaceOrder(PlaceOrderArgs {
        base_quote: btc_usd.clone(),
        user_id: user.user_id.clone(),
        order_uuid: OrderUuid(uuid::Uuid::new_v4()),
        order_details: OrderTicket::builder(
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
        .volume(dec!(0.1))
        .build()
        .unwrap(),
    });

    let resp = {
        let (resp_tx, resp_rx) = oneshot::channel();
        ap_sender
            .send((resp_tx, validate_only_order))
            .await
            .unwrap();
        resp_rx.await.unwrap()
    };
    assert_eq!(resp, Ok(MsgOut::OrderValidated));
}
