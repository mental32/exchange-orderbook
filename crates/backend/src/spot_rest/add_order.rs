use crate::middleware::clerk::Clerk;
use crate::middleware::clerk::ClerkUserId;
use crate::middleware::either::Either;
use crate::middleware::msgpack::Msgpack;
use ap_actor::proc::MsgError;
use ap_actor::proc_router::ProcRouter;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use matching_engine::decimal::Decimal;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::orderbook::SelfTradeProtection;
use matching_engine::orderbook::TimeInForce;
use matching_engine::orderflags::OrderFlags;
use matching_engine::pending_fill::TifViolation;
use matching_engine::price::Price;
use matching_engine::time::Time;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PriceTrigger {
    /// Index price is derived from the sum of the prices from various spot exchanges multiplied by their respective weighage.
    Index,
    /// This is the platform's current market price.
    Last,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TradeAddOrder {
    /// None used in construction of `API-Sign` header.
    pub nonce: u64, // Required
    /// This is an optional non-unique, numeric identifier which can be associated with a number of
    ///
    /// orders by the client. This field is mutually exclusive with `cl_ord_id` parameter.
    /// `userref` is an optional user-specified integer id that can be associated with any number of
    /// orders. Many clients choose a `userref` corresponding to a unique integer id generated
    /// by their systems (e.g. a timestamp). However, because we don't enforce uniqueness on our side,
    /// it can also be used to easily group orders by pair, side, strategy, etc. This allows clients
    /// to more readily cancel or query information about orders in a particular group, with fewer API
    /// calls by using `userref` instead of our `txid`, where supported.
    pub userref: Option<u32>,
    /// Adds an alphanumeric client order identifier which uniquely identifies an open order for each client.
    /// This field is mutually exclusive with `userref` parameter.
    /// The `cl_ord_id` parameter can be one of the following formats:
    /// -   Long UUID: `6d1b345e-2821-40e2-ad83-4ecb18a06876` 32 hex characters separated with 4 dashes.
    /// -   Short UUID: `da8e4ad59b78481c93e589746b0cf91f` 32 hex characters with no dashes.
    /// -   Free text: `arb-20230101-123456` free format ascii text up to 18 characters.
    pub cl_ord_id: Option<String>,
    /// Side of the order.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub side: OrderSide, // Required
    /// The execution model of the order.
    #[cfg_attr(feature = "serde", serde(rename = "ordertype"))]
    pub order_type: OrderType, // Required
    /// Order quantity in terms of the base asset
    /// > Note: Volume can be specified as `0` for closing margin orders to automatically fill
    /// > the requisite quantity.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    #[cfg_attr(feature = "serde", serde(rename = "volume"))]
    pub volume: Decimal, // Required
    /// For `iceberg` orders only, it defines the quantity to show in the book while the rest
    /// of the order quantity remains hidden. Minimum value is 1/15 of `volume`.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "matching_engine::decimal::deserialize_decimal_option",
            serialize_with = "matching_engine::decimal::serialize_decimal_option"
        )
    )]
    #[cfg_attr(feature = "serde", serde(rename = "displayvol"))]
    pub display_volume: Option<Decimal>,
    /// Asset pair `id` or `altname`
    /// Example: "XBTUSD"
    #[cfg_attr(feature = "serde", serde(rename = "pair"))]
    pub symbol: String, // Required
    /// This parameter is required on requests for non-crypto pairs, i.e. use `tokenized_asset` for xstocks.
    pub asset_class: Option<String>,
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
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "matching_engine::price::deserialize_price_option",
            serialize_with = "matching_engine::price::serialize_price_option"
        )
    )]
    pub price: Option<Price>,
    /// Secondary Price:
    ///     -   Limit price for `stop-loss-limit`, `take-profit-limit`, and `trailing-stop-limit` orders
    /// Note:
    ///     -   Trailing Stops: Must use a relative price for this field, namely one of the `+` or `-`
    ///         prefixes. This will provide the offset from the trigger price to the limit price, i.e. +0
    ///         would set the limit price equal to the trigger price. The `%` suffix also works for this field
    ///         to use a relative percentage limit price.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "matching_engine::price::deserialize_price_option",
            serialize_with = "matching_engine::price::serialize_price_option"
        )
    )]
    #[cfg_attr(feature = "serde", serde(rename = "price2"))]
    pub secondary_price: Option<Price>,
    /// Price signal used to trigger `stop-loss`, `stop-loss-limit`, `take-profit`,
    /// `take-profit-limit`, `trailing-stop`, and `trailing-stop-limit` orders.
    /// Notes:
    ///     -   This `trigger` type will also be used for any associated conditional close orders.
    ///     -   To keep triggers servicable, the last price will be used as a fallback reference during connectivity issues with external index feeds.
    #[cfg_attr(feature = "serde", serde(default))]
    pub trigger: Option<PriceTrigger>,
    /// Amount of leverage desired (default: none)
    pub leverage: Option<u32>,
    /// if `true`, order will only reduce a currently open position, not increase it or open a
    /// new position.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reduce_only: bool,
    /// Self Trade Prevention (STP) is a protection feature to prevent users from inadvertently or deliberately trading against themselves.
    /// To prevent a self-match, one of the following STP modes can be used to define which order(s) will be expired:
    /// -   cancel-newest: arriving order will be canceled
    /// -   cancel-oldest: resting order will be canceled
    /// -   cancel-both: both arriving and resting orders will be canceled
    /// Possible values: [cancel-newest, cancel-oldest, cancel-both]
    /// Default value: cancel-newest
    #[cfg_attr(feature = "serde", serde(default, rename = "stptype"))]
    pub self_trade_protection: SelfTradeProtection,
    /// Comma delimited list of order flags
    /// -   `post` post-only order (available when ordertype = limit)
    /// -   `fcib` prefer fee in base currency (default if selling)
    /// -   `fciq` prefer fee in quote currency (default if buying, mutually exclusive with `fcib`)
    /// -   `nompp` (DEPRECATED) -- disabling Market Price Protection for market orders is no longer supported. If supplied, the flag is accepted but ignored.
    /// -   `viqc` order volume expressed in quote currency. This option is supported only for buy market orders. Also not available on margin orders.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "matching_engine::orderflags::deserialize_orderflags_option",
            serialize_with = "matching_engine::orderflags::serialize_orderflags_option",
            rename = "oflags"
        )
    )]
    pub order_flags: Option<OrderFlags>,
    /// Time-in-force specifies how long an order remains in effect before being expired.
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_in_force: TimeInForce,
    /// Scheduled start time, can be specified as an absolute timestamp or as a number of seconds in the future:
    /// -   `0` now (default)
    /// -   `<n>` = unix timestamp of start time
    /// -   `+<n>` = schedule start time <n> seconds from now
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "matching_engine::time", rename = "starttm")
    )]
    pub start_time: Time,
    /// Expiry time on GTD orders can be set up to one month in future, it is specified as an absolute timestamp or as a number of seconds from now:
    /// -   `0` now (default)
    /// -   `<n>` = unix timestamp of expiry time
    /// -   `+<n>` = expiry time <n> seconds from now
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "matching_engine::time", rename = "expiretm")
    )]
    pub expiry_time: Time,
    /// Conditional close order type
    /// > Note: Conditional close orders are triggered by execution of the primary order in
    /// > the same quantity and opposite direction, but once triggered are independent orders
    /// > that may reduce or increase net position.
    #[cfg_attr(feature = "serde", serde(rename = "close[ordertype]"))]
    pub close_order_type: Option<OrderType>,
    /// Conditional close order `price`
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "matching_engine::price::deserialize_price_option",
            serialize_with = "matching_engine::price::serialize_price_option",
            rename = "close[price]"
        )
    )]
    pub close_order_price: Option<Price>,
    /// Conditional close order `price2`
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "matching_engine::price::deserialize_price_option",
            serialize_with = "matching_engine::price::serialize_price_option",
            rename = "close[price2]"
        )
    )]
    pub close_order_price2: Option<Price>,
    /// RFC3339 timestamp (e.g. "2023-01-01T00:18:00Z") after which the matching engine should
    /// reject the new order request, in presence of latency or order queueing: min now() + 2
    /// seconds, max now() + 60 seconds
    #[cfg_attr(feature = "serde", serde(default))]
    pub deadline: Option<String>,
    #[cfg_attr(feature = "serde", serde(rename = "validate"))]
    /// If set to `true` the order will be validated only, it will not trade in the matching engine.
    #[cfg_attr(feature = "serde", serde(default))]
    pub validate_only: Option<bool>,
}

#[cfg(feature = "serde")]
impl From<serde_json::Value> for TradeAddOrder {
    #[track_caller]
    fn from(value: serde_json::Value) -> Self {
        serde_json::from_value(value).expect("deserialize value into TradeAddOrder")
    }
}

pub struct PlaceOrderError(StatusCode, &'static str);

impl From<MsgError> for PlaceOrderError {
    fn from(value: MsgError) -> Self {
        use ap_actor::proc::MsgError as E;
        let (a, b) = match value {
            E::UnserializableInput { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "Invalid input"),
            E::CouldNotPersistToEventSource(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not persist to event source",
            ),
            E::TifViolation(tif_violation) => match tif_violation {
                TifViolation::FillOrKill => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "order details caused Fill or Kill (FOK) violation",
                ),
                TifViolation::ImmediateOrCancel => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "order details caused Immediate or Cancel (IOC) violation",
                ),
            },
            E::ZeroQuantity => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Order quantity must be greater than zero",
            ),
            E::NoReferencePrice => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "No reference price available for relative pricing",
            ),
            E::InsufficientFunds => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Insufficient funds to place order",
            ),
            E::ProcessorIsSuspended => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Trading pair is currently suspended",
            ),
            E::InvalidPrice => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Computed order price is invalid (zero or negative)",
            ),
            E::OrderCancelled => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Order was cancelled during execution",
            ),
            E::NoOpenPositions => (StatusCode::NOT_FOUND, "No open positions for this user"),
            E::OrderNotFound => (StatusCode::NOT_FOUND, "Order not found"),
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
            E::UserAccountNotFound {
                user_id,
                asset_code,
            } => todo!(),
            E::BalanceDatabase(error) => todo!(),
        };

        Self(a, b)
    }
}

impl TradeAddOrder {
    pub fn validate(&self) -> Result<(), (StatusCode, &'static str)> {
        // Phase 1: Basic value validation
        if self.volume <= Decimal::ZERO {
            return Err((StatusCode::BAD_REQUEST, "volume must be greater than zero"));
        }

        if let Some(leverage) = self.leverage {
            if leverage > 5 {
                return Err((StatusCode::UNPROCESSABLE_ENTITY, "leverage cannot exceed 5"));
            }
        }

        if let Some(ref deadline_str) = self.deadline {
            use chrono::DateTime;
            use chrono::Utc;
            let deadline = DateTime::parse_from_rfc3339(deadline_str).map_err(|_| {
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
        }

        // Phase 2: Mutual exclusivity checks
        if self.userref.is_some() && self.cl_ord_id.is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                "userref and cl_ord_id are mutually exclusive",
            ));
        }

        if let Some(ref flags) = self.order_flags {
            if flags.fee_in_base_currency && flags.fee_in_quote_currency {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "fee_in_base_currency and fee_in_quote_currency are mutually exclusive",
                ));
            }
        }

        // Phase 3: Order type specific validation
        match self.order_type {
            OrderType::Limit => {
                if self.price.is_none() {
                    return Err((StatusCode::BAD_REQUEST, "limit orders require price"));
                }
            }
            OrderType::Market => {
                // Market orders don't require price
            }
            OrderType::StopLoss | OrderType::TakeProfit | OrderType::TrailingStop => {
                if self.price.is_none() {
                    return Err((StatusCode::BAD_REQUEST, "triggered orders require price"));
                }
                if self.trigger.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "triggered orders require trigger field",
                    ));
                }
            }
            OrderType::StopLossLimit | OrderType::TakeProfitLimit => {
                if self.price.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "triggered limit orders require price (trigger price)",
                    ));
                }
                if self.secondary_price.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "triggered limit orders require secondary_price (limit price)",
                    ));
                }
                if self.trigger.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "triggered limit orders require trigger field",
                    ));
                }
            }
            OrderType::TrailingStopLimit => {
                if self.price.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "trailing-stop-limit orders require price",
                    ));
                }
                if self.secondary_price.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "trailing-stop-limit orders require secondary_price",
                    ));
                }
                if self.trigger.is_none() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "trailing-stop-limit orders require trigger field",
                    ));
                }

                // Validate price has relative prefix (+) specifically
                if let Some(ref price) = self.price {
                    use matching_engine::price::PricePrefix;
                    match price.prefix {
                        Some(PricePrefix::Plus) => {} // Valid
                        _ => {
                            return Err((
                                StatusCode::BAD_REQUEST,
                                "trailing-stop orders must use + prefix for price",
                            ));
                        }
                    }
                }

                // Validate secondary_price has relative prefix (+/-) - not hash
                if let Some(ref price2) = self.secondary_price {
                    use matching_engine::price::PricePrefix;
                    match price2.prefix {
                        Some(PricePrefix::Plus) | Some(PricePrefix::Sub) => {} // Valid
                        _ => {
                            return Err((
                                StatusCode::BAD_REQUEST,
                                "trailing-stop-limit secondary_price must use +/- prefix (not #)",
                            ));
                        }
                    }
                }
            }
            OrderType::Iceberg => {
                if self.price.is_none() {
                    return Err((StatusCode::BAD_REQUEST, "iceberg orders require price"));
                }

                match self.display_volume {
                    None => {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "iceberg orders require display_volume",
                        ));
                    }
                    Some(display_vol) => {
                        let min_display = self.volume / Decimal::from(15);
                        if display_vol < min_display {
                            return Err((
                                StatusCode::UNPROCESSABLE_ENTITY,
                                "display_volume must be at least 1/15 of volume",
                            ));
                        }
                        if display_vol > self.volume {
                            return Err((
                                StatusCode::UNPROCESSABLE_ENTITY,
                                "display_volume cannot exceed volume",
                            ));
                        }
                    }
                }
            }
        }

        // Phase 4: Time-in-force validation
        match self.time_in_force {
            TimeInForce::GoodTilDate(_) => {
                if matches!(self.expiry_time, Time::Now) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "GTD orders must specify expiry_time (cannot be 0/Now)",
                    ));
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let expiry_ts = self.expiry_time.to_absolute_timestamp(now).ok_or((
                    StatusCode::BAD_REQUEST,
                    "GTD orders must have valid expiry_time",
                ))?;

                const MAX_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days
                if expiry_ts > now + MAX_EXPIRY_SECONDS {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "expiry_time cannot be more than 30 days in the future",
                    ));
                }

                if expiry_ts <= now {
                    return Err((StatusCode::BAD_REQUEST, "expiry_time must be in the future"));
                }
            }
            _ => {
                // Other TIF types shouldn't use GTD variant
            }
        }

        // Phase 5: Order flags compatibility
        if let Some(ref flags) = self.order_flags {
            if flags.post_only && self.order_type != OrderType::Limit {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "post-only flag only valid for limit orders",
                ));
            }

            if flags.volume_in_quote_currency {
                if self.side != OrderSide::Buy {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "volume_in_quote_currency only valid for buy orders",
                    ));
                }
                if self.order_type != OrderType::Market {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "volume_in_quote_currency only valid for market orders",
                    ));
                }
                if self.leverage.is_some() {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "volume_in_quote_currency not available on margin orders",
                    ));
                }
            }
        }

        // Phase 6: Conditional close order validation
        if let Some(close_type) = self.close_order_type {
            match close_type {
                OrderType::Limit => {
                    if self.close_order_price.is_none() {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "conditional close limit orders require close_order_price",
                        ));
                    }
                }
                OrderType::Market => {
                    // Market close orders don't require price
                }
                OrderType::StopLoss | OrderType::TakeProfit => {
                    if self.close_order_price.is_none() {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "conditional close triggered orders require close_order_price",
                        ));
                    }
                }
                OrderType::StopLossLimit | OrderType::TakeProfitLimit => {
                    if self.close_order_price.is_none() {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "conditional close triggered limit orders require close_order_price",
                        ));
                    }
                    if self.close_order_price2.is_none() {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "conditional close triggered limit orders require close_order_price2",
                        ));
                    }
                }
                OrderType::TrailingStop | OrderType::TrailingStopLimit => {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "trailing-stop orders cannot be used as conditional close orders",
                    ));
                }
                OrderType::Iceberg => {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "iceberg orders cannot be used as conditional close orders",
                    ));
                }
            }
        }

        Ok(())
    }
}

impl From<TradeAddOrder> for matching_engine::order_ticket::OrderTicket {
    fn from(order: TradeAddOrder) -> Self {
        // For market orders without price, use sentinel value (ignored by matching engine)
        let price = order.price.unwrap_or_else(|| Price {
            prefix: None,
            amount: Decimal::ONE,
            is_percentage: false,
        });

        matching_engine::order_ticket::OrderTicket {
            order_type: order.order_type,
            side: order.side,
            quantity: NonZeroDecimal::new(order.volume).ok(),
            price,
            time_in_force: order.time_in_force,
            stp: order.self_trade_protection,
            display_quantity: order
                .display_volume
                .and_then(|d| NonZeroDecimal::new(d).ok()),
            userref: order.userref,
            cl_ord_id: order.cl_ord_id,
            expiry_time: order.expiry_time,
            secondary_price: order.secondary_price,
            order_flags: order.order_flags.unwrap_or_default(),
            validate_only: order.validate_only.unwrap_or(false),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TradeAddOrderResponse {
    order_uuid: uuid::Uuid,
}

pub async fn f(
    State(proc_router): State<ProcRouter>,
    State(users): State<crate::Users>,
    State(pg_pool): State<sqlx::PgPool>,
    Extension(clerk): Extension<Clerk>,
    body: Either<Json<TradeAddOrder>, Msgpack<TradeAddOrder>>,
) -> Result<Json<TradeAddOrderResponse>, (StatusCode, &'static str)> {
    let trade_add_order = match body {
        Either::Left(Json(l)) | Either::Right(Msgpack(l)) => l,
    };

    let _span = tracing::info_span!(
        "trade_add_order",
        user_uuid = %clerk.user_id(),
        base_quote = %trade_add_order.symbol,
        side = ?trade_add_order.side,
        order_type = ?trade_add_order.order_type,
        quantity = ?trade_add_order.volume,
        price = ?trade_add_order.price,
        time_in_force = ?trade_add_order.time_in_force,
        stp = ?trade_add_order.self_trade_protection,
    );

    let () = trade_add_order.validate()?;

    let Some(base_quote) = proc_router.is_pair_enabled(&trade_add_order.symbol) else {
        tracing::warn!("asset not enabled");
        return Err((StatusCode::NOT_FOUND, "asset not enabled"));
    };

    tracing::info!("placing order for asset");

    match proc_router
        .place_order(
            base_quote,
            users.to_user_pk(clerk.user_id(), pg_pool).await,
            OrderUuid(uuid::Uuid::new_v4()),
            trade_add_order.into(),
        )
        .await
    {
        Ok(OrderUuid(order_uuid)) => Ok(Json(TradeAddOrderResponse { order_uuid })),
        Err(msg_err) => {
            let PlaceOrderError(a, b) = PlaceOrderError::from(msg_err);
            Err((a, b))
        }
    }
}

#[cfg(all(feature = "serde", test))]
mod test_serde {
    use super::TradeAddOrder;

    #[test]
    fn test_de_trade_add_order() {
        // Test minimal required fields only (fields marked with // Required comments)
        // Only these 5 fields should be required: nonce, type, ordertype, volume, pair
        let json = "{\"nonce\": 123456789, \"type\": \"buy\",\"ordertype\": \"limit\",\"volume\": \"0.1\",\"pair\": \"XBTUSD\"}";

        let order: TradeAddOrder =
            serde_json::from_str(json).expect("deserialize minimal required fields");

        // Verify the required fields were parsed correctly
        assert_eq!(order.nonce, 123456789);
        assert_eq!(order.side, matching_engine::orderbook::OrderSide::Buy);
        assert_eq!(
            order.order_type,
            matching_engine::orderbook::OrderType::Limit
        );
        assert_eq!(order.volume, "0.1".parse().unwrap());
        assert_eq!(order.symbol, "XBTUSD");

        // Optional fields should have default values
        assert_eq!(order.userref, None);
        assert_eq!(order.cl_ord_id, None);
        assert_eq!(order.validate_only, None); // Option<bool> defaults to None
        assert_eq!(order.display_volume, None); // Optional field
        assert_eq!(order.price, None); // Optional field with default price
        assert_eq!(order.order_flags, None); // Optional field defaults to None
    }

    #[test]
    fn test_de_gtd_order_with_expiry() {
        let order: TradeAddOrder = serde_json::from_str(
            "{\"nonce\": 123456789,\"type\": \"buy\",\"ordertype\": \"limit\",\"volume\": \"0.1\",\"pair\": \"BTC/USD\",\"price\": \"50000\",\"timeinforce\": \"GTD\",\"expiretm\": \"+60\"}",
        )
        .expect("deserialize GTD order with expiry");

        assert_eq!(order.nonce, 123456789);
        assert_eq!(order.side, matching_engine::orderbook::OrderSide::Buy);
        assert_eq!(
            order.order_type,
            matching_engine::orderbook::OrderType::Limit
        );
        assert_eq!(order.volume, "0.1".parse().unwrap());
        assert_eq!(order.symbol, "BTC/USD");

        assert_eq!(
            order.expiry_time,
            matching_engine::time::Time::Scheduled(60)
        );
        assert_eq!(
            order.time_in_force,
            matching_engine::orderbook::TimeInForce::GoodTilCanceled
        );
    }
}

#[cfg(test)]
mod test {
    use ap_actor::proc::MsgError;
    use base64ct::Encoding as _;

    use super::*;

    #[test]
    fn test_place_order_error_mappings() {
        // Test ZeroQuantity
        let error = MsgError::ZeroQuantity;
        let PlaceOrderError(status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "Order quantity must be greater than zero");

        // Test NoReferencePrice
        let error = MsgError::NoReferencePrice;
        let PlaceOrderError(status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "No reference price available for relative pricing");

        // Test InsufficientFunds
        let error = MsgError::InsufficientFunds;
        let PlaceOrderError(status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "Insufficient funds to place order");

        // Test ProcessorIsSuspended (503 - the only non-422)
        let error = MsgError::ProcessorIsSuspended;
        let PlaceOrderError(status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(message, "Trading pair is currently suspended");

        // Test InvalidPrice
        let error = MsgError::InvalidPrice;
        let PlaceOrderError(status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            message,
            "Computed order price is invalid (zero or negative)"
        );

        // Test OrderCancelled
        let error = MsgError::OrderCancelled;
        let PlaceOrderError(status, message) = PlaceOrderError::from(error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(message, "Order was cancelled during execution");
    }

    fn minimal_valid_order() -> TradeAddOrder {
        TradeAddOrder {
            nonce: 1,
            userref: None,
            cl_ord_id: None,
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            volume: Decimal::from(1),
            display_volume: None,
            symbol: "BTC/USD".to_owned(),
            asset_class: None,
            price: Some(Price {
                prefix: None,
                amount: Decimal::from(50000),
                is_percentage: false,
            }),
            secondary_price: None,
            trigger: None,
            leverage: None,
            reduce_only: false,
            self_trade_protection: SelfTradeProtection::default(),
            order_flags: None,
            time_in_force: TimeInForce::default(),
            start_time: Time::Now,
            expiry_time: Time::Now,
            close_order_type: None,
            close_order_price: None,
            close_order_price2: None,
            deadline: None,
            validate_only: None,
        }
    }

    #[test]
    fn test_compute_signature_matches_kraken_example() {
        let secret = "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==";
        let secret_bytes = base64ct::Base64::decode_vec(secret).unwrap();

        let uri_path = "/0/private/AddOrder";
        let nonce = "1616492376594";
        let post_data =
            "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25";

        let expected_signature = "4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ==";

        let computed =
            crate::middleware::api_sign::hmac_signature(uri_path, nonce, post_data, &secret_bytes);

        assert_eq!(computed, expected_signature);
    }

    #[test]
    fn test_validate_failure_volume_zero() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.volume = Decimal::ZERO;
        let result = order.validate();
        assert_eq!(
            result,
            Err((StatusCode::BAD_REQUEST, "volume must be greater than zero"))
        );
    }

    #[test]
    fn test_validate_failure_leverage_exceeds_max() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.leverage = Some(6);
        let result = order.validate();
        assert_eq!(
            result,
            Err((StatusCode::UNPROCESSABLE_ENTITY, "leverage cannot exceed 5"))
        );
    }

    #[test]
    fn test_validate_failure_userref_and_cl_ord_id_both_set() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.userref = Some(12345);
        order.cl_ord_id = Some("abc-123".to_owned());
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::BAD_REQUEST,
                "userref and cl_ord_id are mutually exclusive"
            ))
        );
    }

    #[test]
    fn test_validate_failure_fee_flags_both_set() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_flags = Some(OrderFlags {
            post_only: false,
            fee_in_base_currency: true,
            fee_in_quote_currency: true,
            volume_in_quote_currency: false,
        });
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::BAD_REQUEST,
                "fee_in_base_currency and fee_in_quote_currency are mutually exclusive"
            ))
        );
    }

    #[test]
    fn test_validate_failure_limit_order_without_price() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Limit;
        order.price = None;
        let result = order.validate();
        assert_eq!(
            result,
            Err((StatusCode::BAD_REQUEST, "limit orders require price"))
        );
    }

    #[test]
    fn test_validate_failure_stop_loss_without_trigger() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::StopLoss;
        order.trigger = None;
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::BAD_REQUEST,
                "triggered orders require trigger field"
            ))
        );
    }

    #[test]
    fn test_validate_failure_iceberg_without_display_volume() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Iceberg;
        order.display_volume = None;
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::BAD_REQUEST,
                "iceberg orders require display_volume"
            ))
        );
    }

    #[test]
    fn test_validate_failure_iceberg_display_volume_too_small() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Iceberg;
        order.volume = Decimal::from(15);
        order.display_volume = Some(Decimal::new(5, 1)); // 0.5, but minimum is 15/15 = 1
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "display_volume must be at least 1/15 of volume"
            ))
        );
    }

    #[test]
    fn test_validate_failure_iceberg_display_volume_too_large() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Iceberg;
        order.volume = Decimal::from(10);
        order.display_volume = Some(Decimal::from(11));
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "display_volume cannot exceed volume"
            ))
        );
    }

    #[test]
    fn test_validate_failure_trailing_stop_limit_without_plus_prefix() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::TrailingStopLimit;
        order.trigger = Some(PriceTrigger::Last);
        order.price = Some(Price {
            prefix: None, // Should be Plus
            amount: Decimal::from(100),
            is_percentage: false,
        });
        order.secondary_price = Some(Price {
            prefix: Some(matching_engine::price::PricePrefix::Plus),
            amount: Decimal::from(0),
            is_percentage: false,
        });
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::BAD_REQUEST,
                "trailing-stop orders must use + prefix for price"
            ))
        );
    }

    #[test]
    fn test_validate_failure_post_only_on_market_order() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Market;
        order.price = None; // Market orders don't need price
        order.order_flags = Some(OrderFlags {
            post_only: true,
            fee_in_base_currency: false,
            fee_in_quote_currency: false,
            volume_in_quote_currency: false,
        });
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "post-only flag only valid for limit orders"
            ))
        );
    }

    #[test]
    fn test_validate_failure_viqc_on_sell_order() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Market;
        order.side = OrderSide::Sell;
        order.price = None;
        order.order_flags = Some(OrderFlags {
            post_only: false,
            fee_in_base_currency: false,
            fee_in_quote_currency: false,
            volume_in_quote_currency: true,
        });
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "volume_in_quote_currency only valid for buy orders"
            ))
        );
    }

    #[test]
    fn test_validate_failure_conditional_close_as_iceberg() {
        use axum::http::StatusCode;
        let mut order = minimal_valid_order();
        order.close_order_type = Some(OrderType::Iceberg);
        let result = order.validate();
        assert_eq!(
            result,
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "iceberg orders cannot be used as conditional close orders"
            ))
        );
    }

    // Validation success tests

    #[test]
    fn test_validate_success_minimal_limit_order() {
        let order = minimal_valid_order();
        assert_eq!(order.validate(), Ok(()));
    }

    #[test]
    fn test_validate_success_market_order() {
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Market;
        order.price = None; // Market orders don't need price
        assert_eq!(order.validate(), Ok(()));
    }

    #[test]
    fn test_validate_success_stop_loss_with_trigger() {
        let mut order = minimal_valid_order();
        order.order_type = OrderType::StopLoss;
        order.trigger = Some(PriceTrigger::Last);
        assert_eq!(order.validate(), Ok(()));
    }

    #[test]
    fn test_validate_success_iceberg_minimum_display_volume() {
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Iceberg;
        order.volume = Decimal::from(15);
        order.display_volume = Some(Decimal::from(1)); // exactly 15/15 = 1
        assert_eq!(order.validate(), Ok(()));
    }

    #[test]
    fn test_validate_success_iceberg_maximum_display_volume() {
        let mut order = minimal_valid_order();
        order.order_type = OrderType::Iceberg;
        order.volume = Decimal::from(10);
        order.display_volume = Some(Decimal::from(10)); // maximum allowed
        assert_eq!(order.validate(), Ok(()));
    }

    #[test]
    fn test_validate_success_trailing_stop_limit_with_proper_prefixes() {
        let mut order = minimal_valid_order();
        order.order_type = OrderType::TrailingStopLimit;
        order.trigger = Some(PriceTrigger::Last);
        order.price = Some(Price {
            prefix: Some(matching_engine::price::PricePrefix::Plus), // Required: + prefix
            amount: Decimal::from(100),
            is_percentage: false,
        });
        order.secondary_price = Some(Price {
            prefix: Some(matching_engine::price::PricePrefix::Sub), // Allowed: +/- prefix
            amount: Decimal::from(50),
            is_percentage: false,
        });
        assert_eq!(order.validate(), Ok(()));
    }

    #[test]
    fn test_validate_success_conditional_close_stop_loss_limit() {
        let mut order = minimal_valid_order();
        order.close_order_type = Some(OrderType::StopLossLimit);
        order.close_order_price = Some(Price {
            prefix: None,
            amount: Decimal::from(48000),
            is_percentage: false,
        });
        order.close_order_price2 = Some(Price {
            prefix: None,
            amount: Decimal::from(47500),
            is_percentage: false,
        });
        assert_eq!(order.validate(), Ok(()));
    }
}
