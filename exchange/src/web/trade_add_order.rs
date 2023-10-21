use axum::extract::{Json, Path, State};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use super::InternalApiState;
use crate::trading::{OrderSide, OrderType, SelfTradeProtection};
use crate::Asset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAddOrder {
    side: OrderSide,
    order_type: OrderType,
    stp: SelfTradeProtection,
    // timeinforce: TimeInForce,
    // iceberg_order: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct TradeAddOrderResponse {}

/// Place an order for `asset`
pub async fn trade_add_order(
    State(state): State<InternalApiState>,
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

    let TradeAddOrder {
        order_type,
        stp,
        // timeinforce,
        side,
        // iceberg_order,
    } = trade_add_order_body;

    state.app_cx.place_order(asset, order_type, stp, side).await;

    Json(TradeAddOrderResponse {}).into_response()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_serialize_trade_order() {
        const HEX_TRADE_ORDER: &str =
            "84aa6f726465725f74797065a66d61726b6574a3737470a26463ab74696d65696e666f726365a3696f63a473696465a3627579";

        let trade_order = TradeAddOrder {
            order_type: OrderType::Market,
            stp: SelfTradeProtection::DecreaseCancel,
            // timeinforce: TimeInForce::ImmediateOrCancel,
            side: OrderSide::Buy,
            // iceberg_order: None,
        };

        let bytes = hex::decode(&HEX_TRADE_ORDER).unwrap();
        let de_trade_order = rmp_serde::from_slice::<TradeAddOrder>(&bytes).unwrap();

        assert_eq!(trade_order.order_type, de_trade_order.order_type);
        assert_eq!(trade_order.stp, de_trade_order.stp);
        // assert_eq!(trade_order.timeinforce, de_trade_order.timeinforce);
    }
}
