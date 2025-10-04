//! Module for placing orders in the matching engine.
//!
use super::asset_code::AssetCode;
use super::orderbook::Order;
use super::orderbook::OrderIndex;
use super::pending_fill::CommitFillError;
use super::pending_fill::FillType;
use super::timeinforce::TimeInForce;
use super::try_fill_order::try_fill_orders;
use crate::asset_pair::BaseQuote;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::orderbook::Orderbook;
use crate::pending_fill::PendingFill;
use crate::reserve_money::ReserveByAssetError;
use common_core::web::middleware::clerk::ClerkUserId;

pub trait PlaceOrderDetails: Clone {
    fn order_side(&self) -> OrderSide;
    fn quantity(&self) -> Option<NonZeroDecimal>;
    fn price(&self) -> Option<NonZeroDecimal>;
    fn order_type(&self) -> OrderType;
    fn time_in_force(&self) -> TimeInForce;
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceOrderArgs<D> {
    pub base_quote: BaseQuote,
    pub user_id: ClerkUserId,
    pub order_uuid: OrderUuid,
    pub details: D,
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
    ExecutePendingFillError(#[from] CommitFillError),
    /// some asset pair (base/quote) was not found in the system
    #[error("invalid asset pair")]
    InvalidAssetPair,
    /// some error occurred while reserving funds for the order
    #[error("could not reserve funds to place order")]
    ReserveError(#[from] ReserveByAssetError),
}

/// Result of placing an order.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PlaceOrderResult<D> {
    /// original order information
    pub original_args: PlaceOrderArgs<D>,
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
pub fn do_place_order<'ob, D>(
    pending_fill: PendingFill<'ob>,
    place_order: PlaceOrderArgs<D>,
) -> Result<PlaceOrderResult<D>, PlaceOrderError>
where
    D: PlaceOrderDetails,
{
    let PlaceOrderArgs {
        base_quote: _,
        user_id,
        order_uuid,
        details,
    } = &place_order;

    // let quantity = NonZeroDecimal::new(quantity).map_err(|()| PlaceOrderError::ZeroQuantity)?;
    // let price = NonZeroDecimal::new(price).map_err(|()| PlaceOrderError::ZeroPrice)?;

    // let taker: Order = Order {
    //     memo: u32::MAX,
    //     quantity,
    //     price,
    // };

    // // create a pending fill and maybe execute it.
    // let pending_fill =
    //     try_fill_orders(orderbook, taker, side, order_type, stp).expect("todo: handle error");

    // enforce time-in-force depending on fill type.
    match (pending_fill.taker_fill_outcome, details.time_in_force()) {
        (FillType::Complete, _) => (), // do nothing, order was completely filled.
        (FillType::Partial, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
        (FillType::Partial, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
        (FillType::Partial, TimeInForce::ImmediateOrCancel) => (), // commit the partial fill, but do not add to orderbook.
        (FillType::Partial, TimeInForce::FillOrKill) => {
            // there were no resting orders that could be filled against the taker order.
            return Err(PlaceOrderError::FillOrKillFailed.into());
        }
        (FillType::NotFilled, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
        (FillType::NotFilled, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
        (FillType::NotFilled, TimeInForce::ImmediateOrCancel) => {
            // no fill, no orderbook entry, NO SOUP FOR YOU!
            return Err(PlaceOrderError::InsufficientLiquidity.into());
        }
        (FillType::NotFilled, TimeInForce::FillOrKill) => {
            return Err(PlaceOrderError::FillOrKillFailed.into());
        }
    }

    // if validate_only {
    //     return todo!("Validate the order without committing it to the orderbook.");
    // }

    todo!()

    // commit the fill.
    // match pending_fill.commit() {
    //     Ok((fill_type, order)) => {
    //         if let Some(order) = order {
    //             let order_index = if matches!(time_in_force, TimeInForce::ImmediateOrCancel) {
    //                 // partial fill, but we do not add it to the orderbook because it is an IOC order.
    //                 None
    //             } else {
    //                 // order was not completely filled, add it to the orderbook.
    //                 Some(match side {
    //                     OrderSide::Buy => pending_fill.push_bid(order),
    //                     OrderSide::Sell => pending_fill.push_ask(order),
    //                 })
    //             };

    //             assert!(*quantity >= *order.quantity);

    //             Ok(PlaceOrderResult {
    //                 original_args: place_order.clone(),
    //                 order_index,
    //                 fill_type,
    //                 quantity_filled: *quantity - *order.quantity,
    //                 quantity_remaining: *order.quantity,
    //             })
    //         } else {
    //             // order is None means that the order was completely filled.
    //             Ok(PlaceOrderResult {
    //                 original_args: place_order.clone(),
    //                 order_index: None,
    //                 fill_type,
    //                 quantity_filled: *quantity,
    //                 quantity_remaining: Decimal::ZERO,
    //             })
    //         }
    //     }
    //     Err(err) => {
    //         tracing::error!(?err, "failed to commit fill");
    //         todo!();
    //         // Err(TradingEngineError::from(
    //         //     PlaceOrderError::ExecutePendingFillError(err),
    //         // ))
    //     }
    // }
}
