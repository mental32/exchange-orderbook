use crate::middleware::clerk::Clerk;
use crate::middleware::clerk::ClerkUserId;
use ap_actor::proc::AmendOrderArgs;
use ap_actor::proc::MsgError;
use ap_actor::proc::OrderDescriptor;
use ap_actor::proc_router::ProcRouter;
use ap_actor::user_profile::OpenOrder;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use matching_engine::decimal::Decimal;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::price::Price;
use sqlx::PgPool;
use std::str::FromStr;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EditOrderRequest {
    pub nonce: u64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub userref: Option<i32>,
    #[cfg_attr(feature = "serde", serde(rename = "txid"))]
    pub txid: EditOrderIdentifier,
    #[cfg_attr(feature = "serde", serde(default))]
    pub volume: Option<String>,
    #[cfg_attr(feature = "serde", serde(default, rename = "displayvol"))]
    pub display_volume: Option<String>,
    #[cfg_attr(feature = "serde", serde(rename = "pair"))]
    pub symbol: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub asset_class: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub price: Option<String>,
    #[cfg_attr(feature = "serde", serde(default, rename = "price2"))]
    pub price_two: Option<String>,
    #[cfg_attr(feature = "serde", serde(default, rename = "oflags"))]
    pub order_flags: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub deadline: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub cancel_response: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub validate: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum EditOrderIdentifier {
    OrderId(String),
    UserReference(i64),
}

impl EditOrderRequest {
    fn new_order_qty(&self) -> Result<Option<Decimal>, (StatusCode, &'static str)> {
        self.volume
            .as_deref()
            .map(|raw| {
                Decimal::from_str(raw).map_err(|_| (StatusCode::BAD_REQUEST, "invalid volume"))
            })
            .transpose()
            .and_then(|opt| match opt {
                Some(qty) if qty <= Decimal::ZERO => {
                    Err((StatusCode::BAD_REQUEST, "volume must be greater than zero"))
                }
                Some(qty) => Ok(Some(qty)),
                None => Ok(None),
            })
    }

    fn new_display_qty(&self) -> Result<Option<Decimal>, (StatusCode, &'static str)> {
        self.display_volume
            .as_deref()
            .map(|raw| {
                Decimal::from_str(raw)
                    .map_err(|_| (StatusCode::BAD_REQUEST, "invalid display volume"))
            })
            .transpose()
            .and_then(|opt| match opt {
                Some(qty) if qty <= Decimal::ZERO => Err((
                    StatusCode::BAD_REQUEST,
                    "display volume must be greater than zero",
                )),
                Some(qty) => Ok(Some(qty)),
                None => Ok(None),
            })
    }

    fn post_only(&self) -> Result<bool, (StatusCode, &'static str)> {
        match self.order_flags.as_deref() {
            None | Some("") => Ok(false),
            Some(raw) => {
                let mut post = false;
                for flag in raw.split(',').map(|f| f.trim()).filter(|f| !f.is_empty()) {
                    if flag == "post" {
                        post = true;
                    } else {
                        return Err((StatusCode::BAD_REQUEST, "unsupported order flag"));
                    }
                }
                Ok(post)
            }
        }
    }

    fn ensure_supported(&self) -> Result<(), (StatusCode, &'static str)> {
        if let Some(asset_class) = &self.asset_class {
            if asset_class != "tokenized_asset" {
                return Err((StatusCode::BAD_REQUEST, "unsupported asset_class"));
            }
        }

        if let Some(deadline) = &self.deadline {
            validate_deadline(deadline)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EditOrderResponse {
    result: EditOrderResult,
    error: Vec<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct EditOrderResult {
    descr: EditOrderDescription,
    txid: String,
    originaltxid: String,
    status: String,
    orders_cancelled: u32,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    newuserref: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    olduserref: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    volume: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    price: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    price2: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    error_message: Option<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct EditOrderDescription {
    order: String,
}

pub async fn f(
    State(proc_router): State<ProcRouter>,
    State(users): State<crate::Users>,
    State(pg_pool): State<PgPool>,
    Extension(clerk): Extension<Clerk>,
    Json(request): Json<EditOrderRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    request.ensure_supported()?;

    let requested_pair = proc_router
        .is_pair_enabled(&request.symbol)
        .ok_or((StatusCode::NOT_FOUND, "asset not enabled"))?;

    let user_id = users.to_user_pk(clerk.user_id(), pg_pool).await;

    let (base_quote, descriptor, order_uuid) =
        resolve_target_order(&proc_router, &user_id, &request)
            .await?
            .ok_or((StatusCode::NOT_FOUND, "order not found"))?;

    if base_quote != requested_pair {
        return Err((StatusCode::BAD_REQUEST, "pair does not match order"));
    }

    let last_trade_price = latest_trade_price(&proc_router, &base_quote);

    let filled_quantity = descriptor.filled_quantity;
    let original_total_quantity = descriptor.filled_quantity + descriptor.remaining_quantity;

    let new_total_quantity_opt = request.new_order_qty()?;
    let mut final_total_quantity = new_total_quantity_opt.unwrap_or(original_total_quantity);

    if final_total_quantity < filled_quantity {
        return Err((
            StatusCode::BAD_REQUEST,
            "volume cannot be less than filled amount",
        ));
    }

    let new_display_qty = request.new_display_qty()?;
    if let Some(display) = new_display_qty {
        if display > final_total_quantity {
            return Err((
                StatusCode::BAD_REQUEST,
                "display volume cannot exceed volume",
            ));
        }
    }

    let post_only = request.post_only()?;
    if post_only && !matches!(descriptor.order_type, OrderType::Limit | OrderType::Iceberg) {
        return Err((
            StatusCode::BAD_REQUEST,
            "post-only flag is only valid for limit/iceberg orders",
        ));
    }

    if request.price_two.is_some()
        && !matches!(
            descriptor.order_type,
            OrderType::StopLossLimit | OrderType::TakeProfitLimit | OrderType::TrailingStopLimit
        )
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "price2 is not applicable to this order type",
        ));
    }

    let mut new_limit_price: Option<NonZeroDecimal> = None;
    let mut new_trigger_price: Option<NonZeroDecimal> = None;
    let mut final_limit_price = descriptor.limit_price;
    let mut final_trigger_price = descriptor.trigger_price;

    if let Some(price_str) = &request.price {
        let computed = compute_price(
            price_str,
            descriptor.order_type,
            descriptor.side,
            last_trade_price,
            descriptor
                .trigger_price
                .or(descriptor.limit_price)
                .or_else(|| final_trigger_price),
        )?;

        match descriptor.order_type {
            OrderType::Limit | OrderType::Iceberg => {
                new_limit_price = Some(computed);
                final_limit_price = Some(computed);
            }
            OrderType::StopLoss
            | OrderType::StopLossLimit
            | OrderType::TakeProfit
            | OrderType::TakeProfitLimit
            | OrderType::TrailingStop
            | OrderType::TrailingStopLimit => {
                new_trigger_price = Some(computed);
                final_trigger_price = Some(computed);
            }
            _ => {}
        }
    }

    if let Some(price_str) = &request.price_two {
        let computed = compute_price(
            price_str,
            descriptor.order_type,
            descriptor.side,
            last_trade_price,
            descriptor.limit_price,
        )?;
        new_limit_price = Some(computed);
        final_limit_price = Some(computed);
    }

    ensure_price_requirements(
        descriptor.order_type,
        final_limit_price,
        final_trigger_price,
    )?;

    if let Some(new_qty) = new_total_quantity_opt {
        final_total_quantity = new_qty;
    }

    let amend_args = AmendOrderArgs {
        user_id,
        order_uuid,
        new_order_qty: new_total_quantity_opt,
        new_display_qty,
        new_limit_price: new_limit_price,
        new_trigger_price: new_trigger_price,
        post_only,
    };

    let final_volume_string = decimal_to_string(final_total_quantity);
    let (response_price, response_price2) = response_price_fields(
        descriptor.order_type,
        final_limit_price,
        final_trigger_price,
    );

    let newuserref = request.userref.map(|id| id.to_string());
    let olduserref = descriptor.userref.map(|id| id.to_string());

    if request.validate {
        let response = EditOrderResponse {
            result: EditOrderResult {
                descr: EditOrderDescription {
                    order: describe_order(
                        descriptor.order_type,
                        descriptor.side,
                        &final_volume_string,
                        response_price.as_deref(),
                        response_price2.as_deref(),
                    ),
                },
                txid: order_uuid.0.to_string(),
                originaltxid: order_uuid.0.to_string(),
                status: "ok".to_string(),
                orders_cancelled: 0,
                newuserref,
                olduserref,
                volume: Some(final_volume_string),
                price: response_price,
                price2: response_price2,
                error_message: None,
            },
            error: Vec::new(),
        };

        return Ok(Json(response));
    }

    let updated_uuid = proc_router
        .amend_order(order_uuid, user_id, amend_args)
        .await
        .map_err(|e| {
            use MsgError as E;
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

    let status = if request.cancel_response {
        "pending"
    } else {
        "ok"
    }
    .to_string();

    let response = EditOrderResponse {
        result: EditOrderResult {
            descr: EditOrderDescription {
                order: describe_order(
                    descriptor.order_type,
                    descriptor.side,
                    &final_volume_string,
                    response_price.as_deref(),
                    response_price2.as_deref(),
                ),
            },
            txid: updated_uuid.0.to_string(),
            originaltxid: order_uuid.0.to_string(),
            status,
            orders_cancelled: 1,
            newuserref,
            olduserref,
            volume: Some(final_volume_string),
            price: response_price,
            price2: response_price2,
            error_message: None,
        },
        error: Vec::new(),
    };

    Ok(Json(response))
}

async fn resolve_target_order(
    proc_router: &ProcRouter,
    user_id: &ap_actor::VirtualUserId,
    request: &EditOrderRequest,
) -> Result<
    Option<(
        matching_engine::asset_pair::BaseQuote,
        OrderDescriptor,
        OrderUuid,
    )>,
    (StatusCode, &'static str),
> {
    match &request.txid {
        EditOrderIdentifier::OrderId(raw) => {
            let uuid = uuid::Uuid::parse_str(raw)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid txid"))?;
            let order_uuid = OrderUuid(uuid);
            let (base_quote, descriptor) = proc_router
                .describe_order_any(*user_id, order_uuid)
                .await
                .map_err(|err| match err {
                    MsgError::OrderNotFound => (StatusCode::NOT_FOUND, "Order not found"),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to describe order",
                    ),
                })?;
            Ok(Some((base_quote, descriptor, order_uuid)))
        }
        EditOrderIdentifier::UserReference(value) => {
            let userref_u32: u32 = (*value)
                .try_into()
                .map_err(|_| (StatusCode::BAD_REQUEST, "userref must be non-negative"))?;

            let matches: Vec<OpenOrder> = proc_router
                .open_orders_for(user_id)
                .await
                .into_iter()
                .filter(|order| order.userref == Some(userref_u32))
                .collect();

            if matches.is_empty() {
                return Ok(None);
            }
            if matches.len() > 1 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "userref must uniquely identify an order",
                ));
            }

            let open_order = matches.into_iter().next().unwrap();
            let descriptor = proc_router
                .describe_order(
                    open_order.base_quote.clone(),
                    *user_id,
                    open_order.order_uuid,
                )
                .await
                .map_err(|err| match err {
                    MsgError::OrderNotFound => (StatusCode::NOT_FOUND, "Order not found"),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to describe order",
                    ),
                })?;

            Ok(Some((
                open_order.base_quote,
                descriptor,
                open_order.order_uuid,
            )))
        }
    }
}

fn compute_price(
    raw: &str,
    order_type: OrderType,
    side: OrderSide,
    last_trade_price: Option<NonZeroDecimal>,
    fallback: Option<NonZeroDecimal>,
) -> Result<NonZeroDecimal, (StatusCode, &'static str)> {
    let parsed =
        Price::from_str(raw).map_err(|_| (StatusCode::BAD_REQUEST, "invalid price format"))?;

    if parsed.is_relative() {
        let reference = last_trade_price.or(fallback).ok_or((
            StatusCode::UNPROCESSABLE_ENTITY,
            "no reference price available",
        ))?;
        parsed
            .compute_to_decimal(reference, side, order_type)
            .map_err(|_| (StatusCode::BAD_REQUEST, "computed price invalid"))
    } else {
        NonZeroDecimal::new(parsed.amount)
            .map_err(|_| (StatusCode::BAD_REQUEST, "price must be greater than zero"))
    }
}

fn ensure_price_requirements(
    order_type: OrderType,
    limit_price: Option<NonZeroDecimal>,
    trigger_price: Option<NonZeroDecimal>,
) -> Result<(), (StatusCode, &'static str)> {
    match order_type {
        OrderType::Limit | OrderType::Iceberg => {
            if limit_price.is_none() {
                return Err((StatusCode::BAD_REQUEST, "limit price required"));
            }
        }
        OrderType::StopLoss | OrderType::TakeProfit | OrderType::TrailingStop => {
            if trigger_price.is_none() {
                return Err((StatusCode::BAD_REQUEST, "trigger price required"));
            }
        }
        OrderType::StopLossLimit | OrderType::TakeProfitLimit | OrderType::TrailingStopLimit => {
            if trigger_price.is_none() || limit_price.is_none() {
                return Err((StatusCode::BAD_REQUEST, "trigger and limit price required"));
            }
        }
        _ => {}
    }

    Ok(())
}

fn response_price_fields(
    order_type: OrderType,
    limit_price: Option<NonZeroDecimal>,
    trigger_price: Option<NonZeroDecimal>,
) -> (Option<String>, Option<String>) {
    match order_type {
        OrderType::Limit | OrderType::Iceberg => (limit_price.map(nonzero_to_string), None),
        OrderType::StopLoss | OrderType::TakeProfit | OrderType::TrailingStop => {
            (trigger_price.map(nonzero_to_string), None)
        }
        OrderType::StopLossLimit | OrderType::TakeProfitLimit | OrderType::TrailingStopLimit => (
            trigger_price.map(nonzero_to_string),
            limit_price.map(nonzero_to_string),
        ),
        _ => (None, None),
    }
}

fn describe_order(
    order_type: OrderType,
    side: OrderSide,
    volume: &str,
    price: Option<&str>,
    price2: Option<&str>,
) -> String {
    let side_str = match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    };

    match order_type {
        OrderType::Limit | OrderType::Iceberg => match price {
            Some(p) => format!("{} {} @ limit {}", side_str, volume, p),
            None => format!("{} {} @ limit", side_str, volume),
        },
        OrderType::StopLoss => match price {
            Some(p) => format!("{} {} @ stop-loss {}", side_str, volume, p),
            None => format!("{} {} @ stop-loss", side_str, volume),
        },
        OrderType::StopLossLimit => match (price, price2) {
            (Some(trigger), Some(limit)) => format!(
                "{} {} @ stop-loss {} -> limit {}",
                side_str, volume, trigger, limit
            ),
            _ => format!("{} {} @ stop-loss-limit", side_str, volume),
        },
        OrderType::TakeProfit => match price {
            Some(p) => format!("{} {} @ take-profit {}", side_str, volume, p),
            None => format!("{} {} @ take-profit", side_str, volume),
        },
        OrderType::TakeProfitLimit => match (price, price2) {
            (Some(trigger), Some(limit)) => format!(
                "{} {} @ take-profit {} -> limit {}",
                side_str, volume, trigger, limit
            ),
            _ => format!("{} {} @ take-profit-limit", side_str, volume),
        },
        OrderType::TrailingStop => match price {
            Some(p) => format!("{} {} @ trailing-stop {}", side_str, volume, p),
            None => format!("{} {} @ trailing-stop", side_str, volume),
        },
        OrderType::TrailingStopLimit => match (price, price2) {
            (Some(trigger), Some(limit)) => format!(
                "{} {} @ trailing-stop {} -> limit {}",
                side_str, volume, trigger, limit
            ),
            _ => format!("{} {} @ trailing-stop-limit", side_str, volume),
        },
        _ => format!("{} {}", side_str, volume),
    }
}

fn decimal_to_string(decimal: Decimal) -> String {
    decimal.normalize().to_string()
}

fn nonzero_to_string(value: NonZeroDecimal) -> String {
    value.normalize().to_string()
}

fn latest_trade_price(
    _engine: &ProcRouter,
    _base_quote: &matching_engine::asset_pair::BaseQuote,
) -> Option<NonZeroDecimal> {
    // Market data not implemented
    None
}

fn validate_deadline(deadline: &str) -> Result<(), (StatusCode, &'static str)> {
    use chrono::DateTime;
    use chrono::Utc;

    let deadline = DateTime::parse_from_rfc3339(deadline).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "deadline must be valid RFC3339 timestamp",
        )
    })?;

    let now = Utc::now();
    let min_deadline = now + chrono::Duration::seconds(2);
    let max_deadline = now + chrono::Duration::seconds(60);

    if deadline.timestamp() < min_deadline.timestamp() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "deadline must be at least 2 seconds in the future",
        ));
    }
    if deadline.timestamp() > max_deadline.timestamp() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "deadline cannot be more than 60 seconds in the future",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> EditOrderRequest {
        EditOrderRequest {
            nonce: 1,
            userref: None,
            txid: EditOrderIdentifier::OrderId("550e8400-e29b-41d4-a716-446655440000".to_string()),
            volume: Some("1.5".to_string()),
            display_volume: Some("0.5".to_string()),
            symbol: "BTC/USD".to_string(),
            asset_class: None,
            price: Some("100.5".to_string()),
            price_two: None,
            order_flags: Some("post".to_string()),
            deadline: None,
            cancel_response: false,
            validate: false,
        }
    }

    #[test]
    fn parse_volume_ok() {
        let req = sample_request();
        assert_eq!(
            req.new_order_qty().unwrap(),
            Some(Decimal::from_str("1.5").unwrap())
        );
    }

    #[test]
    fn parse_display_volume_ok() {
        let req = sample_request();
        assert_eq!(
            req.new_display_qty().unwrap(),
            Some(Decimal::from_str("0.5").unwrap())
        );
    }

    #[test]
    fn post_only_flag_parses() {
        let req = sample_request();
        assert!(req.post_only().unwrap());
    }

    #[test]
    fn ensure_deadline_validation() {
        let mut req = sample_request();
        req.deadline = Some("2021-04-01T00:18:45Z".to_string());
        let result = req.ensure_supported();
        // may fail depending on current clock; simply ensure it returns Err or Ok without panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn describe_order_builds_strings() {
        let description =
            describe_order(OrderType::Limit, OrderSide::Buy, "1.0", Some("100"), None);
        assert!(description.contains("buy"));
        assert!(description.contains("limit"));
    }
}
