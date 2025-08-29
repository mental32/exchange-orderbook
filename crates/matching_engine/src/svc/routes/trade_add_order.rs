use crate::decimal::Decimal;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::place_order::PlaceOrderResult;
use crate::self_trade_protection::SelfTradeProtection;
use crate::svc::ap_actor::MessageResult;
use crate::svc::engine::MatchingEngineFacade;
use crate::timeinforce::TimeInForce;
use axum::Extension;
use axum::extract::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Response;
use common_core::web::internal_server_error;
use common_core::web::middleware::clerk::Clerk;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TradeAddOrder {
    /// The side of the order.
    pub side: OrderSide,
    /// The type of the order.
    pub order_type: OrderType,
    /// The quantity of the order.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub quantity: Decimal,
    /// The price of the order.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub price: Decimal,
    /// The time in force of the order.
    #[serde(default)]
    pub time_in_force: TimeInForce,
    /// The self-trade protection of the order.
    #[serde(default)]
    pub stp: SelfTradeProtection,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TradeAddOrderResponse {
    order_uuid: uuid::Uuid,
}

pub async fn handler(
    State(engine): State<MatchingEngineFacade>,
    Extension(clerk): Extension<Clerk>,
    Path(base_quote): Path<String>,
    Json(body): Json<TradeAddOrder>,
) -> Response {
    let _span = tracing::info_span!(
        "trade_add_order",
        // user_uuid = %user_uuid,
        base_quote = %base_quote,
        side = ?body.side,
        order_type = ?body.order_type,
        quantity = %body.quantity,
        price = %body.price,
        time_in_force = ?body.time_in_force,
        stp = ?body.stp,
    );

    let Some(base_quote) = engine.is_pair_enabled(base_quote) else {
        tracing::warn!("asset not enabled");
        return (axum::http::StatusCode::NOT_FOUND, "asset not enabled").into_response();
    };

    tracing::info!("placing order for asset");

    let (response, reserved_funds) =
        match engine.place_order(base_quote, clerk.user_id(), body).await {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!(?err, "failed to place order");
                return internal_server_error("failed to place order");
            }
        };

    let _deferred_revert =
        reserved_funds.defer_revert(tokio::runtime::Handle::current(), engine.db());

    match response.await {
        Ok(t) => match t {
            Ok(res) => match res {
                Some(message_result) => match message_result {
                    MessageResult::PlaceOrder(PlaceOrderResult { order_uuid, .. }) => {
                        _deferred_revert.cancel();

                        tracing::info!(?order_uuid, "order placed");
                        Json(TradeAddOrderResponse {
                            order_uuid: order_uuid.0,
                        })
                        .into_response()
                    }
                    other => {
                        tracing::warn!(?other, "unexpected message result");
                        return internal_server_error("unexpected message result");
                    }
                },
                None => {
                    tracing::warn!("order was not placed, no result");
                    return internal_server_error("order was not placed");
                }
            },
            Err(err) => {
                tracing::warn!(?err, "failed to place order");
                return internal_server_error("failed to place order");
            }
        },
        Err(err) => {
            tracing::warn!(?err, "engine unresponsive");
            return internal_server_error("engine unresponsive");
        }
    }
}
