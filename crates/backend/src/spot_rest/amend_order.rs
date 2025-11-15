use crate::middleware::clerk::Clerk;
use ap_actor::order_management::OrderManagement;
use ap_actor::proc::AmendOrderArgs;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use sqlx::PgPool;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AmendOrder {
    pub nonce: u64,
    pub txid: Option<uuid::Uuid>,
    pub cl_ord_id: Option<String>,
    pub order_qty: Option<String>,
    pub display_qty: Option<String>,
    pub limit_price: Option<String>,
    pub trigger_price: Option<String>,
    pub pair: Option<String>,
    pub post_only: Option<bool>,
    pub deadline: Option<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AmendOrderResponse {
    pub amend_id: uuid::Uuid,
}

pub async fn f(
    State(engine): State<OrderManagement>,
    State(users): State<crate::Users>,
    State(pg_pool): State<PgPool>,
    Extension(clerk): Extension<Clerk>,
    Json(request): Json<AmendOrder>,
) -> Result<Json<AmendOrderResponse>, (StatusCode, &'static str)> {
    use matching_engine::decimal::Decimal;
    use matching_engine::decimal::NonZeroDecimal;
    use matching_engine::order_uuid::OrderUuid;

    // Validate that txid is provided
    let order_uuid = request
        .txid
        .ok_or((StatusCode::BAD_REQUEST, "txid is required"))?;

    // Parse optional quantity parameters
    let new_order_qty = request
        .order_qty
        .as_ref()
        .map(|s| {
            s.parse::<Decimal>()
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid order_qty"))
        })
        .transpose()?;

    let new_display_qty = request
        .display_qty
        .as_ref()
        .map(|s| {
            s.parse::<Decimal>()
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid display_qty"))
        })
        .transpose()?;

    // Parse optional price parameters
    let new_limit_price = request
        .limit_price
        .as_ref()
        .map(|s| {
            s.parse::<Decimal>()
                .and_then(|d| {
                    NonZeroDecimal::new(d)
                        .map_err(|_| rust_decimal::Error::ErrorString("zero price".into()))
                })
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid limit_price"))
        })
        .transpose()?;

    let new_trigger_price = request
        .trigger_price
        .as_ref()
        .map(|s| {
            s.parse::<Decimal>()
                .and_then(|d| {
                    NonZeroDecimal::new(d)
                        .map_err(|_| rust_decimal::Error::ErrorString("zero price".into()))
                })
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid trigger_price"))
        })
        .transpose()?;

    let post_only = request.post_only.unwrap_or(false);
    let user_id = users.to_user_pk(clerk.user_id(), pg_pool).await;

    let amend_args = AmendOrderArgs {
        user_id,
        order_uuid: OrderUuid(order_uuid),
        new_order_qty,
        new_display_qty,
        new_limit_price,
        new_trigger_price,
        post_only,
    };

    let amended_uuid = engine
        .amend_order(OrderUuid(order_uuid), user_id, amend_args)
        .await
        .map_err(|e| {
            use ap_actor::proc::MsgError as E;
            match e {
                E::OrderNotFound => (StatusCode::NOT_FOUND, "Order not found"),
                E::InvalidPrice => (StatusCode::UNPROCESSABLE_ENTITY, "Invalid price"),
                E::InsufficientFunds => (StatusCode::UNPROCESSABLE_ENTITY, "Insufficient funds"),
                E::CannotReduceBelowFilled => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Cannot reduce order quantity below filled amount",
                ),
                E::PostOnlyWouldCross => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Amendment would cross spread with post_only flag",
                ),
                E::InvalidDisplayQuantity => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Display quantity must be >= 1/15 of remaining quantity",
                ),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
            }
        })?;

    Ok(Json(AmendOrderResponse {
        amend_id: amended_uuid.0,
    }))
}
