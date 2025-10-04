//! Pending fill operations on the [`Orderbook`].

use super::orderbook::Order;
use super::orderbook::OrderIndex;
use super::orderbook::OrderSide;
use super::orderbook::OrderType;
use super::orderbook::Orderbook;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::place_order::PlaceOrderDetails;
use thiserror::Error;

/// An error that can occur when executing a pending fill operation.
#[derive(Debug, Error)]
pub enum CommitFillError {
    /// The order index is invalid.
    #[error("invalid order index")]
    InvalidOrderIndex(OrderIndex),
}

/// The outcome of a fill operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FillType {
    /// The order was completely filled.
    Complete,
    /// The order was partially filled.
    Partial { fill_amount: NonZeroDecimal },
    /// The order was not filled.
    NotFilled,
}

/// a potential fill result from a maker order
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct Fill {
    pub oix: OrderIndex,
    pub order: Order,
    pub fill_type: FillType,
}

/// A pending fill operation on the [`Orderbook`].
pub struct PendingFill<'a> {
    /// capturing the orderbook by mutable reference enforces that the data in the pending-fill does not drift from the
    /// orderbook data.
    pub orderbook: &'a mut Orderbook,
    /// The maker orders that were filled from this taker fill operation.
    pub fills: Vec<Fill>,
    /// The outcome of the fill operation for the taker's order.
    pub taker_fill_outcome: FillType,
}

impl<'a> PendingFill<'a> {
    pub fn abort(self) {
        // unneeded but included the explicit call to drop for clarity
        std::mem::drop(self);
    }

    pub fn commit(self) -> Result<(), CommitFillError> {
        assert!(
            self.fills
                .iter()
                .all(|fill| self.orderbook.get_mut(fill.oix).is_some()),
            "invariant: all computed fills WILL exist in the orderbook on commit"
        );

        Ok(todo!())
        // let mut taker_order_remaining_quantity = *self.taker.quantity;
        // for fill in &self.maker_fills {
        //     if self.orderbook.get_mut(fill.oix).is_none() {
        //         return Err(ExecutePendingFillError::InvalidOrderIndex(fill.oix));
        //     }
        // }
        // for MakerFill {
        //     oix,
        //     maker: order,
        //     fill_type,
        //     ..
        // } in self.maker_fills
        // {
        //     match fill_type {
        //         // complete fill for a maker order.
        //         FillType::Complete => {
        //             let maker_order = self
        //                 .orderbook
        //                 .remove(oix)
        //                 .ok_or(ExecutePendingFillError::InvalidOrderIndex(oix))?; // this should never fail because we already checked that the order exists.
        //             assert_eq!(maker_order, order);
        //             // if this also filled the taker order, then we wont loop again.
        //             taker_order_remaining_quantity -= *maker_order.quantity;
        //         }
        //         // partial fill for a maker order also means a complete fill for the taker order.
        //         FillType::Partial => {
        //             let maker_order = self
        //                 .orderbook
        //                 .get_mut(oix)
        //                 .ok_or(ExecutePendingFillError::InvalidOrderIndex(oix))?; // this should never fail because we already checked that the order exists.
        //             assert_eq!(*maker_order, order);
        //             assert!(taker_order_remaining_quantity < *maker_order.quantity);
        //             maker_order
        //                 .quantity
        //                 .write(*maker_order.quantity - taker_order_remaining_quantity)
        //                 .expect("partial fills of maker orders will always have a quantity greater than zero");
        //             taker_order_remaining_quantity = Decimal::ZERO;
        //         }
        //         FillType::None => unreachable!(),
        //     }
        // }

        // match self.taker_fill_outcome {
        //     FillType::Complete => assert_eq!(taker_order_remaining_quantity, rust_decimal::dec!(0)),
        //     FillType::Partial => {
        //         assert!(*self.taker.quantity > taker_order_remaining_quantity)
        //     }
        //     FillType::None => assert_eq!(taker_order_remaining_quantity, *self.taker.quantity),
        // }

        // let taker_order = if taker_order_remaining_quantity != rust_decimal::dec!(0) {
        //     // the taker order was partially filled.
        //     let mut taker_order = self.taker;
        //     taker_order
        //         .quantity
        //         .write(taker_order_remaining_quantity)
        //         .expect(
        //             "partial fills of taker orders will always have a quantity greater than zero",
        //         );
        //     Some(taker_order)
        // } else {
        //     // the taker order was completely filled.
        //     None
        // };

        // Ok((self.taker_fill_outcome, taker_order))
    }
}
