use axum::extract::{Json, Path, State};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use serde::{Deserialize, Serialize};

use super::middleware::auth::UserUuid;
use super::InternalApiState;
use crate::asset::ContainsAsset as _;
use crate::Asset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeCancelOrder {
    pub order_uuid: uuid::Uuid,
}

#[derive(Debug, Serialize)]
pub struct TradeCancelOrderResponse {}

/// Place an order for `asset`
pub async fn trade_cancel_order(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_uuid)): Extension<UserUuid>,
    Path(asset): Path<String>,
    Json(body): Json<TradeCancelOrder>,
) -> Response {
    let asset = match asset.as_str() {
        "btc" | "BTC" => Asset::Bitcoin,
        "eth" | "ETH" => Asset::Ether,
        _ => {
            tracing::warn!(?asset, "invalid asset");
            return (axum::http::StatusCode::NOT_FOUND, "invalid asset").into_response();
        }
    };

    if !state
        .assets
        .contains_asset(&crate::asset::AssetKey::ByValue(asset))
    {
        tracing::warn!(?asset, "asset not enabled");
        return (axum::http::StatusCode::NOT_FOUND, "asset not enabled").into_response();
    } else {
        tracing::info!(?asset, "placing order for asset");
    }

    let Ok(wait_response) = state.cancel_order(user_uuid, body.order_uuid).await else {
        tracing::warn!("failed to cancel order, trade engine is suspended");
        return super::internal_server_error("trading engine is suspended");
    };

    let Some(res) = wait_response.wait().await else {
        tracing::warn!("wait_response did not return a result");
        return super::internal_server_error("trading engine is unresponsive");
    };

    match res {
        Ok(()) => {
            tracing::info!("order cancelled");
            (axum::http::StatusCode::OK, "order cancelled").into_response()
        }
        Err(err) => {
            tracing::warn!(?err, "failed to cancel order");
            super::internal_server_error("failed to cancel order")
        }
    }
}
