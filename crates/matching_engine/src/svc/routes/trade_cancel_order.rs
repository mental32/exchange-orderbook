use crate::order_uuid::OrderUuid;
use crate::svc::engine::EngineFacade;
use axum::Extension;
use axum::extract::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use common_core::web::internal_server_error;
use common_core::web::middleware::clerk::Clerk;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TxId {
    OrderIdentifier(uuid::Uuid),
    Userref(u64),
}

#[cfg_attr(test, test)]
fn test_de_txid() {
    let json = "\"6b9a6095-0838-40d8-8ecb-7555c1cf7ff2\"";
    let txid = serde_json::from_str::<TxId>(json).unwrap();
    assert!(matches!(txid, TxId::OrderIdentifier(_)));

    let json = r#"123123"#;
    let txid = serde_json::from_str::<TxId>(json).unwrap();
    assert!(matches!(txid, TxId::Userref(_)));
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeCancelOrder {
    pub nonce: u64,
    pub txid: TxId,
    pub cl_ord_id: String,
}

#[cfg_attr(test, test)]
fn test_de_trade_cancel_order() {
    let json =
        r#"{"nonce": 123, "txid": "6b9a6095-0838-40d8-8ecb-7555c1cf7ff2", "cl_ord_id": "123"}"#;
    let order = serde_json::from_str::<TradeCancelOrder>(json).unwrap();
    assert_eq!(order.nonce, 123);
    assert!(matches!(order.txid, TxId::OrderIdentifier(_)));
    assert_eq!(order.cl_ord_id, "123");
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeCancelOrderResponse {
    #[cfg_attr(feature = "serde", serde(rename = "c"))]
    cancelled: Vec<uuid::Uuid>,
    #[cfg_attr(feature = "serde", serde(rename = "f"))]
    failed: Vec<(uuid::Uuid, String)>,
}

pub async fn f(
    State(engine): State<EngineFacade>,
    Extension(clerk): Extension<Clerk>,
    Json(body): Json<TradeCancelOrder>,
) -> Result<Json<TradeCancelOrderResponse>, (StatusCode, &'static str)> {
    use crate::svc::ap_actor::Error as E;
    use crate::svc::ap_actor::MessageOut as R;

    // let Ok(resp) = engine
    //     .cancel_order(OrderUuid(order_uuid.clone()), clerk.user_id())
    //     .await
    // else {
    //     tracing::warn!("failed to cancel order, trade engine is suspended");
    //     return Err((
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         "trading engine is suspended",
    //     ));
    // };

    // let Ok((output, errors)) = resp.await else {
    //     tracing::warn!("wait_response did not return a result");
    //     response
    //         .failed
    //         .push((*order_uuid, "internal error".to_string()));
    //     continue;
    // };

    // match (&*output, &*errors) {
    //     ([Some(R::CancelOrder(maybe_order))], []) => match maybe_order {
    //         Some(order) => {
    //             tracing::info!(?order, "order cancelled");
    //             response.cancelled.push(order_uuid.clone());
    //         }
    //         None => {
    //             tracing::warn!("order not found");
    //             response
    //                 .failed
    //                 .push((*order_uuid, "order not found".to_string()));
    //         }
    //     },
    //     ([], [E::Suspended]) => {
    //         tracing::warn!("trading engine is suspended");
    //         return Err((
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             "trading engine is suspended",
    //         ));
    //     }
    //     ([], [error]) => {
    //         tracing::warn!(?error, "failed to cancel order");
    //         response
    //             .failed
    //             .push((*order_uuid, "internal error".to_string()));
    //     }
    //     _ => {
    //         unreachable!("indicating bug")
    //     }
    // }

    Ok(Json(todo!()))
}
