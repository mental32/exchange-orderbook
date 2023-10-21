use std::num::NonZeroU32;

use thiserror::Error;

use super::*;

#[derive(Debug, Error)]
pub enum ExecutePendingFillError {
    #[error("invalid order index")]
    InvalidOrderIndex(OrderIndex),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillType {
    /// The order was completely filled.
    Complete,
    /// The order was partially filled.
    Partial,
    /// The order was not filled.
    None,
}

/// A pending fill operation on the [`Orderbook`].
pub struct PendingFill<'a> {
    // capturing the orderbook by mutable reference enforces that the data in the pending-fill does not drift from the orderbook data.
    orderbook: &'a mut Orderbook,
    /// The taker's order.
    taker: Order,
    /// The side of the taker's order.
    taker_side: OrderSide,
    /// The order type of the taker's order.
    taker_order_type: OrderType,
    /// The maker orders that were filled from this taker fill operation.
    maker_fills: Vec<(OrderIndex, Order, FillType)>,
    /// The outcome of the fill operation for the taker's order.
    taker_fill_outcome: FillType,
}

impl<'a> PendingFill<'a> {
    pub fn new(
        orderbook: &'a mut Orderbook,
        taker: Order,
        taker_side: OrderSide,
        taker_order_type: OrderType,
        maker_fills: Vec<(OrderIndex, Order, FillType)>,
        taker_fill_outcome: FillType,
    ) -> Self {
        Self {
            orderbook,
            taker,
            taker_side,
            taker_order_type,
            maker_fills,
            taker_fill_outcome,
        }
    }

    /// [`OrderSide::Buy`] or [`OrderSide::Sell`] if the taker's order is buy or sell respectively.
    pub fn taker_side(&self) -> OrderSide {
        self.taker_side
    }

    /// Returns the order type of the taker's order.
    pub fn taker_order_type(&self) -> OrderType {
        self.taker_order_type
    }

    /// Returns the outcome of the fill operation.
    pub fn taker_fill_outcome(&self) -> FillType {
        self.taker_fill_outcome
    }

    /// Abort the pending fill operation.
    pub fn abort(self) {
        // Do nothing and drop the reference to the orderbook.
    }

    /// Execute the pending fill operation.
    pub fn commit(self) -> Result<(FillType, Option<Order>), ExecutePendingFillError> {
        let mut taker_order_remaining_quantity = self.taker.quantity.get();

        for (oix, order, fill_type) in dbg!(self.maker_fills) {
            match fill_type {
                // complete fill for a maker order.
                FillType::Complete => {
                    let maker_order = self
                        .orderbook
                        .remove(oix)
                        .ok_or(ExecutePendingFillError::InvalidOrderIndex(oix))?;
                    assert_eq!(maker_order, order);
                    // if this also filled the taker order, then we wont loop again.
                    taker_order_remaining_quantity -= maker_order.quantity.get();
                }
                // partial fill for a maker order also means a complete fill for the taker order.
                FillType::Partial => {
                    let maker_order = self
                        .orderbook
                        .get_mut(oix)
                        .ok_or(ExecutePendingFillError::InvalidOrderIndex(oix))?;
                    assert_eq!(*maker_order, order);
                    assert!(taker_order_remaining_quantity < maker_order.quantity.get());
                    maker_order.quantity =
                    NonZeroU32::new(maker_order.quantity.get() - taker_order_remaining_quantity).expect("partial fills of maker orders will always have a quantity greater than zero");
                    taker_order_remaining_quantity = 0;
                }
                FillType::None => unreachable!(),
            }
        }

        match self.taker_fill_outcome {
            FillType::Complete => assert_eq!(taker_order_remaining_quantity, 0),
            FillType::Partial => {
                assert!(self.taker.quantity.get() > taker_order_remaining_quantity)
            }
            FillType::None => assert_eq!(taker_order_remaining_quantity, self.taker.quantity.get()),
        }

        let taker_order = if let Some(quantity) = NonZeroU32::new(taker_order_remaining_quantity) {
            let mut taker_order = self.taker;
            taker_order.quantity = quantity;
            Some(taker_order)
        } else {
            // the taker order was completely filled.
            None
        };

        Ok((self.taker_fill_outcome, taker_order))
    }
}
