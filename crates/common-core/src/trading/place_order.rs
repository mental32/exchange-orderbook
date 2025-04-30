use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::trading::{order_uuid::OrderUuid, orderbook::OrderSide};

use super::{
    asset::{Asset, Assets},
    orderbook::{Order, OrderIndex, OrderType},
    pending_fill::{ExecutePendingFillError, FillType},
    self_trade_protection::SelfTradeProtection,
    timeinforce::TimeInForce,
    try_fill_order::try_fill_orders,
};

/// Data for placing an order.
#[derive(Debug, Deserialize, Serialize)]
pub struct PlaceOrder {
    /// the asset to trade
    asset: Asset,
    /// the user that placed the order
    user_uuid: uuid::Uuid,
    /// the price of the order
    price: NonZeroU32,
    /// the quantity of the order
    quantity: NonZeroU32,
    /// the type of order
    order_type: OrderType,
    /// the self trade protection setting
    stp: SelfTradeProtection,
    /// the time in force setting
    time_in_force: TimeInForce,
    /// the side of the order, buy or sell
    side: OrderSide,
}

impl PlaceOrder {
    /// create a new [`PlaceOrder``]
    pub fn new(
        asset: Asset,
        user_uuid: uuid::Uuid,
        price: NonZeroU32,
        quantity: NonZeroU32,
        order_type: OrderType,
        stp: SelfTradeProtection,
        time_in_force: TimeInForce,
        side: OrderSide,
    ) -> Self {
        Self {
            asset,
            user_uuid,
            price,
            quantity,
            order_type,
            stp,
            time_in_force,
            side,
        }
    }
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
}

/// Result of placing an order.
pub struct PlaceOrderResult {
    // original order information
    /// the asset to trade
    pub asset: Asset,
    /// the user that placed the order
    pub user_uuid: uuid::Uuid,
    /// the price of the order
    pub price: NonZeroU32,
    /// the quantity of the order
    pub quantity: NonZeroU32,
    /// the type of order
    pub order_type: OrderType,
    /// the self trade protection setting
    pub stp: SelfTradeProtection,
    /// the time in force setting
    pub time_in_force: TimeInForce,
    /// the side of the order, buy or sell
    pub side: OrderSide,
    // result of the order
    /// the unique identifier for the order
    pub order_uuid: OrderUuid,
    /// the index of the order in the orderbook
    pub order_index: Option<OrderIndex>,
    /// the type of fill that occurred
    pub fill_type: FillType,
    /// the quantity filled
    pub quantity_filled: u32,
    /// the quantity remaining
    pub quantity_remaining: u32,
}

/// place an order
pub fn do_place_order(
    assets: &mut Assets,
    place_order: PlaceOrder,
) -> Result<PlaceOrderResult, PlaceOrderError> {
    let PlaceOrder {
        asset,
        user_uuid,
        price,
        quantity,
        order_type,
        stp,
        time_in_force,
        side,
    } = place_order;

    let asset_book = assets.match_asset_mut(asset);

    let taker: Order = Order {
        memo: u32::MAX,
        quantity,
        price,
    };

    // create a pending fill and maybe execute it.
    let pending_fill = try_fill_orders(asset_book.orderbook_mut(), taker, side, order_type)
        .expect("todo: handle error");

    // TODO: self trade protection

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
                        OrderSide::Buy => asset_book.orderbook_mut().push_bid(order),
                        OrderSide::Sell => asset_book.orderbook_mut().push_ask(order),
                    })
                };

                assert!(quantity.get() >= order.quantity.get());

                Ok(PlaceOrderResult {
                    asset,
                    user_uuid,
                    order_index,
                    price,
                    quantity,
                    order_type,
                    stp,
                    time_in_force,
                    side,
                    order_uuid: OrderUuid::new_v4(),
                    fill_type,
                    quantity_filled: quantity.get() - order.quantity.get(),
                    quantity_remaining: order.quantity.get(),
                })
            } else {
                // order is None means that the order was completely filled.
                Ok(PlaceOrderResult {
                    asset,
                    user_uuid,
                    order_index: None,
                    price,
                    quantity,
                    order_type,
                    stp,
                    time_in_force,
                    side,
                    order_uuid: OrderUuid::new_v4(),
                    fill_type,
                    quantity_filled: quantity.get(),
                    quantity_remaining: 0,
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
