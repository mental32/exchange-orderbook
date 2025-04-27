use std::num::NonZeroU32;

use axum::extract::{Json, Path, State};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use serde::{Deserialize, Serialize};

use super::middleware::auth::UserUuid;
use super::InternalApiState;
use crate::asset::ContainsAsset as _;
use crate::trading::{
    OrderSide, OrderType, PlaceOrderResult, SelfTradeProtection, TimeInForce,
    TradingEngineError as TErr,
};
use crate::Asset;

/// The request body for the `trade_add_order` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAddOrder {
    /// The side of the order.
    pub side: OrderSide,
    /// The type of the order.
    pub order_type: OrderType,
    /// The quantity of the order.
    pub quantity: NonZeroU32,
    /// The price of the order.
    pub price: NonZeroU32,
    /// The time in force of the order.
    #[serde(default)]
    pub time_in_force: TimeInForce,
    /// The self-trade protection of the order.
    #[serde(default)]
    pub stp: SelfTradeProtection,
}

/// The response body for the `trade_add_order` endpoint.
#[derive(Debug, Serialize)]
pub struct TradeAddOrderResponse {
    order_uuid: uuid::Uuid,
}

/// Place an order for `asset`
pub async fn f(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_uuid)): Extension<UserUuid>,
    Path(asset): Path<String>,
    Json(body): Json<TradeAddOrder>,
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

    let (response, reserved_funds) = match state.place_order(asset, user_uuid, body).await {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(?err, "failed to place order");
            return super::internal_server_error("failed to place order");
        }
    };

    let _deferred_revert =
        reserved_funds.defer_revert(tokio::runtime::Handle::current(), state.db());

    let order_uuid = response.wait().await;

    if matches!(order_uuid, Some(Ok(_))) {
        _deferred_revert.cancel();
    }

    match order_uuid {
        Some(Ok(PlaceOrderResult { order_uuid, .. })) => {
            tracing::info!(?order_uuid, "order placed");
            Json(TradeAddOrderResponse {
                order_uuid: order_uuid.0,
            })
            .into_response()
        }
        Some(Err(err)) => match err {
            TErr::UnserializableInput => super::internal_server_error(
                "this input was considered problematic and could not be processed",
            ),
            err => {
                tracing::warn!(?err, "failed to place order");
                super::internal_server_error("failed to place order")
            }
        },
        None => {
            tracing::warn!("trading engine unresponsive");
            super::internal_server_error("trading engine unresponsive")
        }
    }
}
