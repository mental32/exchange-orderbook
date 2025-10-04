use crate::conditional::ConditionalParameters;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::place_order::PlaceOrderDetails;
use crate::place_order::PlaceOrderResult;
use crate::self_trade_protection::SelfTradeProtection;
use crate::svc::ap_actor::MessageOut;
use crate::svc::engine::EngineFacade;
use crate::timeinforce::TimeInForce;
use axum::Extension;
use axum::extract::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use common_core::web::internal_server_error;
use common_core::web::middleware::clerk::Clerk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PriceTrigger {
    Index,
    Last,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TradeAddOrder {
    /// Side of the order.
    pub side: OrderSide,
    /// The execution model of the order.
    pub order_type: OrderType,
    /// Order quantity in terms of the base asset.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    #[cfg_attr(feature = "serde", serde(rename = "order_qty"))]
    pub quantity: Decimal,
    /// The symbol of the currency pair.
    /// Example: "BTC/USD"
    pub symbol: String,
    /// - Limit price for `limit` and `iceberg` orders.
    /// - Trigger price for `stop-loss`, `stop-loss-limit`, `take-profit`, `take-profit-limit`,
    ///     `trailing-stop`, `trailing-stop-limit` orders.
    /// Notes:
    ///     -   Relative prices: either `price` or `price2` can be preceded by `+`, `-`, or `#`
    ///         to specify the order price as an offset relative to the last traded price. `+`
    ///         adds the amount to, and `-` subtracts the amount from the last traded price. `#`
    ///         will either add or subtract the amount to the last traded price, depending on the
    ///         direction and order type used. Prices can also be suffixed with a `%` to signify
    ///         the relative amount as a percentage, rather than an absolute price difference.
    ///     -   Trailing Stops: Must use a relative price for this field, namely the `+` prefix,
    ///         from which the direction will be automatic based on if the original order is a buy
    ///         or sell (no need to use `-` or `#`).
    ///         The `%` suffix also works for these order types to use a relative percentage price.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub price: Decimal,
    /// Secondary Price:
    ///     -   Limit price for `stop-loss-limit`, `take-profit-limit`, and `trailing-stop-limit` orders
    /// Note:
    ///     -   Trailing Stops: Must use a relative price for this field, namely one of the `+` or `-`
    ///         prefixes. This will provide the offset from the trigger price to the limit price, i.e. +0
    ///         would set the limit price equal to the trigger price. The `%` suffix also works for this field
    ///         to use a relative percentage limit price.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    #[cfg_attr(feature = "serde", serde(rename = "price2"))]
    pub secondary_price: Decimal,
    /// The conditional parameters are used as a template for generating the secondary close orders when the primary
    /// order fills. Each fill on the primary order will generate a new secondary order. The size of the secondary
    /// order will be the same size as the executed quantity and have the opposite side.
    pub conditional: Option<ConditionalParameters>,
    /// Defines the quantity to show in the book while the rest of order quantity remains hidden.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    #[cfg_attr(feature = "serde", serde(rename = "display_qty"))]
    pub display_quantity: Decimal,
    /// Time-in-force specifies how long an order remains in effect before being expired.
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_in_force: TimeInForce,
    /// Self Trade Prevention (STP) is a protection feature to prevent users from inadvertently or deliberately trading against themselves.
    /// To prevent a self-match, one of the following STP modes can be used to define which order(s) will be expired:
    /// - cancel-newest: arriving order will be canceled
    /// - cancel-oldest: resting order will be canceled
    /// - cancel-both: both arriving and resting orders will be canceled
    /// Possible values: [cancel-newest, cancel-oldest, cancel-both]
    /// Default value: cancel-newest
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(rename = "stp_type"))]
    pub stp: SelfTradeProtection,
    #[cfg_attr(feature = "serde", serde(rename = "validate"))]
    /// If set to `true` the order will be validated only, it will not trade in the matching engine.
    pub validate_only: bool,
    /// Price signal used to trigger `stop-loss`, `stop-loss-limit`, `take-profit`,
    /// `take-profit-limit`, `trailing-stop`, and `trailing-stop-limit` orders.
    /// Notes:
    ///     -   This `trigger` type will also be used for any associated conditional close orders.
    ///     -   To keep triggers servicable, the last price will be used as a fallback reference during connectivity issues with external index feeds.
    // #[cfg_attr(feature = "serde", serde(default))]
    pub trigger: PriceTrigger,
}

impl PlaceOrderDetails for TradeAddOrder {
    fn order_side(&self) -> OrderSide {
        self.side
    }

    fn quantity(&self) -> Option<crate::decimal::NonZeroDecimal> {
        NonZeroDecimal::new(self.quantity).ok()
    }

    fn price(&self) -> Option<crate::decimal::NonZeroDecimal> {
        NonZeroDecimal::new(self.price).ok()
    }

    fn order_type(&self) -> OrderType {
        self.order_type
    }

    fn time_in_force(&self) -> TimeInForce {
        self.time_in_force
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TradeAddOrderResponse {
    order_uuid: uuid::Uuid,
}

pub async fn f(
    State(engine): State<EngineFacade>,
    Extension(clerk): Extension<Clerk>,
    Json(body): Json<TradeAddOrder>,
) -> Result<Json<TradeAddOrderResponse>, (StatusCode, &'static str)> {
    let _span = tracing::info_span!(
        "trade_add_order",
        user_uuid = %clerk.user_id(),
        base_quote = %body.symbol,
        side = ?body.side,
        order_type = ?body.order_type,
        quantity = %body.quantity,
        price = %body.price,
        time_in_force = ?body.time_in_force,
        stp = ?body.stp,
    );

    let Some(base_quote) = engine.is_pair_enabled(&body.symbol) else {
        tracing::warn!("asset not enabled");
        return Err((StatusCode::NOT_FOUND, "asset not enabled"));
    };

    tracing::info!("placing order for asset");

    match engine.place_order(base_quote, clerk.user_id(), body).await {
        Ok(OrderUuid(order_uuid)) => Ok(Json(TradeAddOrderResponse { order_uuid })),
        Err(err) => {
            tracing::warn!(?err, "failed to place order");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to place order"));
        }
    }
}
