//! Module for placing orders in the matching engine.
//!
use super::asset_code::AssetCode;
use super::orderbook::Order;
use super::orderbook::OrderIndex;
use super::orderbook::OrderType;
use super::pending_fill::ExecutePendingFillError;
use super::pending_fill::FillType;
use super::self_trade_protection::SelfTradeProtection;
use super::timeinforce::TimeInForce;
use super::try_fill_order::try_fill_orders;
use crate::decimal::Decimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderSide;
use crate::orderbook::Orderbook;
use crate::svc::engine::ReserveByAssetError;
use common_core::web::middleware::clerk::ClerkUserId;

/// Data for placing an order.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceOrder {
    /// the asset to trade
    pub base_quote: (AssetCode, AssetCode),
    /// the user that placed the order
    pub user_id: ClerkUserId,
    /// the price of the order
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub price: Decimal,
    /// the quantity of the order
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub quantity: Decimal,
    /// the type of order
    pub order_type: OrderType,
    /// the self trade protection setting
    pub stp: SelfTradeProtection,
    /// the time in force setting
    pub time_in_force: TimeInForce,
    /// the side of the order, buy or sell
    pub side: OrderSide,
}

/// Error that can occur when placing an order.
#[derive(Debug, thiserror::Error)]
pub enum PlaceOrderError {
    /// error that can occur when executing a pending fill operation.
    #[error("order was not completely filled due to insufficient liquidity")]
    FillOrKillFailed,
    /// error that can occur when executing a pending fill operation.
    #[error("order was not completely filled due to insufficient liquidity")]
    InsufficientLiquidity,
    /// error that can occur when executing a pending fill operation.
    #[error("error while executing pending fill")]
    ExecutePendingFillError(#[from] ExecutePendingFillError),
    /// some asset pair (base/quote) was not found in the system
    #[error("invalid asset pair")]
    InvalidAssetPair,
    /// some error occurred while reserving funds for the order
    #[error("reserve error")]
    ReserveError(#[from] ReserveByAssetError),
}

/// Result of placing an order.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PlaceOrderResult {
    /// original order information
    pub request: PlaceOrder,
    // --- result of the order
    /// the unique identifier for the order
    pub order_uuid: OrderUuid,
    /// the index of the order in the orderbook
    pub order_index: Option<OrderIndex>,
    /// the type of fill that occurred
    pub fill_type: FillType,
    /// the quantity filled
    pub quantity_filled: Decimal,
    /// the quantity remaining
    pub quantity_remaining: Decimal,
}

/// place an order
pub fn do_place_order(
    orderbook: &mut Orderbook,
    place_order: PlaceOrder,
) -> Result<PlaceOrderResult, PlaceOrderError> {
    let PlaceOrder {
        base_quote: _,
        user_id: _,
        price,
        quantity,
        order_type,
        stp,
        time_in_force,
        side,
    } = place_order.clone();

    let taker: Order = Order {
        memo: u32::MAX,
        quantity,
        price,
    };

    // create a pending fill and maybe execute it.
    let pending_fill =
        try_fill_orders(orderbook, taker, side, order_type).expect("todo: handle error");

    crate::self_trade_protection::self_trade_protection(&pending_fill, stp);

    // enforce time-in-force depending on fill type.
    match (pending_fill.taker_fill_outcome(), time_in_force) {
        (FillType::Complete, _) => (), // do nothing, order was completely filled.
        (FillType::Partial, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
        (FillType::Partial, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
        (FillType::Partial, TimeInForce::ImmediateOrCancel) => (), // commit the partial fill, but do not add to orderbook.
        (FillType::Partial, TimeInForce::FillOrKill) => {
            // there were no resting orders that could be filled against the taker order.
            return Err(PlaceOrderError::FillOrKillFailed.into());
        }
        (FillType::None, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
        (FillType::None, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
        (FillType::None, TimeInForce::ImmediateOrCancel) => {
            // no fill, no orderbook entry, NO SOUP FOR YOU!
            return Err(PlaceOrderError::InsufficientLiquidity.into());
        }
        (FillType::None, TimeInForce::FillOrKill) => {
            return Err(PlaceOrderError::FillOrKillFailed.into());
        }
    }

    // commit the fill.
    match pending_fill.commit() {
        Ok((fill_type, order)) => {
            if let Some(order) = order {
                let order_index = if matches!(time_in_force, TimeInForce::ImmediateOrCancel) {
                    // partial fill, but we do not add it to the orderbook because it is an IOC order.
                    None
                } else {
                    // order was not completely filled, add it to the orderbook.
                    Some(match side {
                        OrderSide::Buy => orderbook.push_bid(order),
                        OrderSide::Sell => orderbook.push_ask(order),
                    })
                };

                assert!(quantity >= order.quantity);

                Ok(PlaceOrderResult {
                    request: place_order,
                    order_index,
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    fill_type,
                    quantity_filled: quantity - order.quantity,
                    quantity_remaining: order.quantity,
                })
            } else {
                // order is None means that the order was completely filled.
                Ok(PlaceOrderResult {
                    request: place_order,
                    order_index: None,
                    order_uuid: OrderUuid(uuid::Uuid::new_v4()),
                    fill_type,
                    quantity_filled: quantity,
                    quantity_remaining: Decimal::ZERO,
                })
            }
        }
        Err(err) => {
            tracing::error!(?err, "failed to commit fill");
            todo!();
            // Err(TradingEngineError::from(
            //     PlaceOrderError::ExecutePendingFillError(err),
            // ))
        }
    }
}
