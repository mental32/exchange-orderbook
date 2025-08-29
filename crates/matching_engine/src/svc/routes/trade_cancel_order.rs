use crate::order_uuid::OrderUuid;
use crate::svc::engine::MatchingEngineFacade;
use axum::Extension;
use axum::extract::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Response;
use common_core::web::internal_server_error;
use common_core::web::middleware::clerk::Clerk;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeCancelOrder {
    pub orders: Vec<OrderDetails>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderDetails {
    pub order_uuid: uuid::Uuid,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeCancelOrderResponse {
    #[cfg_attr(feature = "serde", serde(rename = "c"))]
    cancelled: Vec<uuid::Uuid>,
    #[cfg_attr(feature = "serde", serde(rename = "f"))]
    failed: Vec<(uuid::Uuid, String)>,
}

/// Cancel an order
pub async fn handler(
    State(engine): State<MatchingEngineFacade>,
    Extension(clerk): Extension<Clerk>,
    Path(base_quote): Path<String>,
    Json(body): Json<TradeCancelOrder>,
) -> Response {
    use crate::svc::ap_actor::Error as E;
    use crate::svc::ap_actor::MessageResult as R;

    let Some(_base_quote) = engine.is_pair_enabled(base_quote) else {
        tracing::warn!("asset not enabled");
        return (axum::http::StatusCode::NOT_FOUND, "asset not enabled").into_response();
    };

    let mut failed = vec![];
    let mut cancelled = vec![];

    for OrderDetails { order_uuid } in &body.orders {
        let Ok(wait_response) = engine
            .cancel_order(OrderUuid(order_uuid.clone()), clerk.user_id())
            .await
        else {
            tracing::warn!("failed to cancel order, trade engine is suspended");
            return internal_server_error("trading engine is suspended");
        };

        let Ok(res) = wait_response.await else {
            tracing::warn!("wait_response did not return a result");
            failed.push((*order_uuid, "internal error".to_string()));
            continue;
        };

        match res {
            Ok(Some(R::CancelOrder(maybe_order))) => match maybe_order {
                Some(order) => {
                    tracing::info!(?order, "order cancelled");
                    cancelled.push(*order_uuid);
                }
                None => {
                    tracing::warn!("order not found");
                    failed.push((*order_uuid, "order not found".to_string()));
                }
            },
            Ok(Some(ref _res)) => {
                tracing::warn!(?res, "unexpected response from cancel_order");
                failed.push((*order_uuid, "internal error".to_string()));
            }
            Ok(None) => {
                unreachable!("indicates bug");
            }
            Err(E::Suspended(msg)) => {
                tracing::warn!(?msg, "trading engine is suspended");
                return internal_server_error("trading engine is suspended");
            }
            Err(_err) => {
                tracing::warn!(?_err, "failed to cancel order");
                failed.push((*order_uuid, "internal error".to_string()));
            }
        }
    }

    Json(TradeCancelOrderResponse {
        cancelled: body.orders.iter().map(|o| o.order_uuid).collect(),
        failed: vec![],
    })
    .into_response()
}
