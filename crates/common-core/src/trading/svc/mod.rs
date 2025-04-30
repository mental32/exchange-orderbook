mod routes;

pub async fn serve(cx: Cx) -> anyhow::Result<()> {
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use tokio::task_local;
//     use uuid::Uuid;

//     use crate::Configuration;
//     use crate::spawn_trading_engine::{SpawnTradingEngine, spawn_trading_engine};

//     use super::*;

//     task_local! {
//         static CX: (TradingEngineTx, sqlx::PgPool);
//     }

//     async fn trading_engine_fixture(db: sqlx::PgPool) -> (Configuration, SpawnTradingEngine) {
//         let config = crate::config::Configuration::load_from_toml("");
//         let spawn_trading_engine = spawn_trading_engine(&config, db);
//         (config, spawn_trading_engine)
//     }

//     async fn place_order(user_uuid: Uuid, price: u32, quantity: u32) -> PlaceOrderResult {
//         let (te, db) = CX.with(|cx| cx.clone());

//         let (tx, rx) = oneshot::channel();
//         let order = PlaceOrder {
//             asset: Asset::Bitcoin,
//             user_uuid: Uuid::new_v4(),
//             price: NonZeroU32::new(price).expect("price was zero"),
//             quantity: NonZeroU32::new(quantity).expect("quantity was zero"),
//             order_type: OrderType::Market,
//             stp: SelfTradeProtection::CancelOldest,
//             time_in_force: TimeInForce::GoodTilCanceled,
//             side: OrderSide::Buy,
//         };

//         te.send(TradingEngineCmd::Trade(TradeCmd::PlaceOrder((order, tx))))
//             .await
//             .expect("place-order send error");

//         rx.await
//             .expect("oneshot rx failure")
//             .expect("place-order Err")
//     }

//     #[sqlx::test]
//     async fn test_startup_then_shutdown(db: sqlx::PgPool) {
//         let (_config, te) = trading_engine_fixture(db).await;
//         te.input.send(TradingEngineCmd::Shutdown).await.unwrap();
//         te.handle.await.unwrap();
//     }

//     pub fn new_user_uuid() -> Uuid {
//         Uuid::new_v4()
//     }

//     #[sqlx::test(migrations = "../../migrations")]
//     async fn test_place_order(db: sqlx::PgPool) {
//         let _ = tracing_subscriber::fmt().with_test_writer().try_init();

//         let (_config, te) = trading_engine_fixture(db.clone()).await;
//         let (te, _task) = te.init_from_db(db.clone()).await.unwrap();
//         CX.scope((te, db), async {
//             let users = (0..100).map(|_| new_user_uuid()).collect::<Vec<_>>();
//             let bob = users[0];

//             let PlaceOrderResult { asset, .. } = place_order(bob, 1, 1).await;
//             assert_eq!(asset, Asset::Bitcoin);
//         });
//     }
// }
