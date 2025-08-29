use crate::asset_code::AssetCode;
use crate::decimal::Decimal;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::place_order::PlaceOrder;
use crate::place_order::PlaceOrderResult;
use crate::self_trade_protection::SelfTradeProtection;
use crate::svc::engine::MatchingEngineFacade;
use crate::timeinforce::TimeInForce;
use axum::Router;
use common_core::configuration::Configuration;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

//     use super::*;

//     task_local! {
//         static CX: (TradingEngineTx, sqlx::PgPool);
//     }

async fn engine_fixture(db: sqlx::PgPool) -> (Configuration, Router) {
    let config = Configuration::from_str("");
    let app = common_core::web::apply_middleware(super::routes::routes(MatchingEngineFacade {
        pool: db.clone(),
        ap_list: todo!(),
        order_uuids: todo!(),
    }));
    (config, app)
}

async fn place_order(
    router: Router,
    user_uuid: Uuid,
    price: Decimal,
    quantity: Decimal,
) -> PlaceOrderResult {
    let order = PlaceOrder {
        base_quote: (
            AssetCode::from_str("BTC").unwrap(),
            AssetCode::from_str("USD").unwrap(),
        ),
        user_id: Uuid::new_v4(),
        price: price,
        quantity: quantity,
        order_type: OrderType::Market,
        stp: SelfTradeProtection::CancelOldest,
        time_in_force: TimeInForce::GoodTilCanceled,
        side: OrderSide::Buy,
    };

    let req = axum::http::Request::builder().body(String::new()).unwrap();

    let res = router.oneshot(req).await.unwrap();

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let body: PlaceOrderResult = serde_json::from_slice(&body).unwrap();
    body
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_place_order(db: sqlx::PgPool) {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let (c, te) = engine_fixture(db.clone()).await;
    //         let (te, _task) = te.init_from_db(db.clone()).await.unwrap();
    //         CX.scope((te, db), async {
    //             let users = (0..100).map(|_| new_user_uuid()).collect::<Vec<_>>();
    //             let bob = users[0];

    //             let PlaceOrderResult { asset, .. } = place_order(bob, 1, 1).await;
    //             assert_eq!(asset, Asset::Bitcoin);
    //         });
}
