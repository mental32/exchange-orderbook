use std::num::NonZeroU32;

use axum::extract::{Json, Path, State};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use serde::{Deserialize, Serialize};

use super::middleware::auth::UserUuid;
use super::InternalApiState;
use crate::trading::{
    OrderSide, OrderType, OrderUuid, SelfTradeProtection, TimeInForce, TradingEngineError as TErr,
};
use crate::Asset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAddOrder {
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: NonZeroU32,
    pub price: NonZeroU32,
    #[serde(default)]
    pub time_in_force: TimeInForce,
    #[serde(default)]
    pub stp: SelfTradeProtection,
}

#[derive(Debug, Serialize)]
pub struct TradeAddOrderResponse {
    order_uuid: uuid::Uuid,
}

/// Place an order for `asset`
pub async fn trade_add_order(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_uuid)): Extension<UserUuid>,
    Path(asset): Path<String>,
    Json(trade_add_order_body): Json<TradeAddOrder>,
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
        .contains_key(&crate::asset::AssetKey::ByValue(asset))
    {
        tracing::warn!(?asset, "asset not enabled");
        return (axum::http::StatusCode::NOT_FOUND, "asset not enabled").into_response();
    } else {
        tracing::info!(?asset, "placing order for asset");
    }

    let response = match state
        .app_cx
        .place_order(asset, user_uuid, trade_add_order_body)
        .await
    {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(?err, "failed to place order");
            return super::internal_server_error("failed to place order");
        }
    };

    match response.wait().await {
        Some(Ok(OrderUuid(order_uuid))) => {
            tracing::info!(?order_uuid, "order placed");
            Json(TradeAddOrderResponse { order_uuid }).into_response()
        }
        Some(Err(err)) => match err {
            TErr::Suspended => {
                tracing::warn!("trading engine suspended");
                super::internal_server_error("trading engine suspended")
            }
            TErr::OrderNotFound(_, _) => {
                unreachable!()
            }
            TErr::UnserializableInput => super::internal_server_error(
                "this input was considered problematic and could not be processed",
            ),
            TErr::Database(_) => super::internal_server_error("database error"),
        },
        None => {
            tracing::warn!("trading engine unresponsive");
            super::internal_server_error("trading engine unresponsive")
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    pub fn test_serialize_trade_order() {
        const HEX_TRADE_ORDER: &str =
            "84aa6f726465725f74797065a66d61726b6574a3737470a26463ab74696d65696e666f726365a3696f63a473696465a3627579";

        let trade_order = TradeAddOrder {
            order_type: OrderType::Market,
            stp: SelfTradeProtection::DecreaseCancel,
            time_in_force: TimeInForce::ImmediateOrCancel,
            quantity: NonZeroU32::new(100).unwrap(),
            price: NonZeroU32::new(100).unwrap(),
            side: OrderSide::Buy,
        };

        let bytes = hex::decode(&HEX_TRADE_ORDER).unwrap();
        let de_trade_order = rmp_serde::from_slice::<TradeAddOrder>(&bytes).unwrap();

        assert_eq!(trade_order.order_type, de_trade_order.order_type);
        assert_eq!(trade_order.stp, de_trade_order.stp);
        // assert_eq!(trade_order.timeinforce, de_trade_order.timeinforce);
    }
}
