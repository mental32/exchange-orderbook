//! Pending fill operations on the [`Orderbook`].

use super::orderbook::OrderData;
use super::orderbook::OrderIndex;
use super::orderbook::OrderSide;
use super::orderbook::OrderType;
use super::orderbook::Orderbook;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::orderbook::TimeInForce;
use crate::place_order::OrderDetails;

/// The outcome of a fill operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FillType {
    /// The order was completely filled.
    Complete,
    /// The order was partially filled.
    Partial { amount_filled: NonZeroDecimal },
    /// The order was not filled.
    NotFilled,
    /// The order was cancelled due to self-trade protection.
    Cancelled,
}

/// a potential fill result from a maker order
#[derive(Debug, PartialEq, Eq)]
pub struct Fill {
    pub order_index: OrderIndex,
    pub fill_type: FillType,
}

/// A pending fill operation on the [`Orderbook`].
pub struct PendingFill<'a> {
    /// capturing the orderbook by mutable reference enforces that the data in the pending-fill does not drift from the
    /// orderbook data.
    pub orderbook: &'a mut Orderbook,
    /// The orders that were filled
    pub fills: Vec<Fill>,
    /// The outcome of the fill operation for the taker's order.
    pub taker_fill_outcome: FillType,
}

#[derive(Debug, thiserror::Error)]
pub enum TifViolation {
    /// FillOrKill order was not completely filled (got partial or no fill)
    #[error("FillOrKill order was not completely filled")]
    FillOrKill,
    /// ImmediateOrCancel order could not fill any quantity
    #[error("ImmediateOrCancel order had no liquidity available")]
    ImmediateOrCancel,
}

impl<'a> PendingFill<'a> {
    pub fn check_tif(&self, time_in_force: TimeInForce) -> Result<(), TifViolation> {
        // enforce time-in-force depending on fill type.
        match (self.taker_fill_outcome, time_in_force) {
            (FillType::Complete, _) => (), // do nothing, order was completely filled.
            (FillType::Partial { .. }, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
            (FillType::Partial { .. }, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
            (FillType::Partial { .. }, TimeInForce::ImmediateOrCancel) => (), // commit the partial fill, but do not add to orderbook.
            (FillType::Partial { .. }, TimeInForce::FillOrKill) => {
                // FillOrKill requires complete fill, but order was only partially filled
                return Err(TifViolation::FillOrKill.into());
            }
            (FillType::NotFilled, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
            (FillType::NotFilled, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
            (FillType::NotFilled, TimeInForce::ImmediateOrCancel) => {
                // ImmediateOrCancel requires at least partial fill, but no liquidity was available
                return Err(TifViolation::ImmediateOrCancel.into());
            }
            (FillType::NotFilled, TimeInForce::FillOrKill) => {
                return Err(TifViolation::FillOrKill.into());
            }
            (FillType::Cancelled, _) => {
                // Cancelled orders don't participate in TIF checks
                // This shouldn't happen for taker orders, only for resting orders
            }
        }

        Ok(())
    }

    pub fn abort(self) {
        std::mem::drop(self); // included call to drop for clarity
    }

    pub fn commit(self) -> &'a mut Orderbook {
        assert!(
            self.fills
                .iter()
                .all(|fill| self.orderbook.get_mut(fill.order_index).is_some()),
            "invariant: all computed fills WILL exist in the orderbook on commit"
        );

        for Fill {
            order_index: oix,
            fill_type,
        } in self.fills.into_iter()
        {
            match fill_type {
                FillType::Complete => {
                    let order = self.orderbook.get_mut(oix).expect("always valid");

                    // For Iceberg orders, "Complete" means display_quantity was filled
                    if let Some(display_qty) = order.display_quantity {
                        // Reduce remaining_quantity by the display amount
                        let new_remaining = *order.remaining_quantity - *display_qty;

                        if let Ok(new_remaining_nz) = NonZeroDecimal::new(new_remaining) {
                            // Still have hidden reserve - replenish display and keep order in book
                            order.remaining_quantity = new_remaining_nz;
                            // Replenish display_quantity up to the new remaining amount
                            order.display_quantity = Some(new_remaining_nz.min(display_qty));

                            // invariant: display_quantity must never exceed remaining_quantity
                            assert!(
                                order.display_quantity.unwrap_or_else(|| {
                                    // Create a dummy NonZeroDecimal for the else case
                                    NonZeroDecimal::new(crate::decimal::Decimal::ONE).unwrap()
                                }) <= order.remaining_quantity,
                                "invariant: display_quantity cannot exceed remaining_quantity after fill"
                            );
                        } else {
                            // No reserve left - remove the order completely
                            self.orderbook.remove(oix);
                        }
                    } else {
                        // Regular order - remove it completely
                        self.orderbook.remove(oix);
                    }
                }
                FillType::Partial { amount_filled } => {
                    let order = self.orderbook.get_mut(oix).expect("always valid");

                    // For both Iceberg and regular orders, reduce the quantities
                    let new_remaining = *order.remaining_quantity - *amount_filled;
                    order.remaining_quantity = NonZeroDecimal::new(new_remaining)
                        .expect("remaining quantity after partial fill must be non-zero");

                    // For Iceberg orders, also reduce the display_quantity
                    if let Some(display_qty) = order.display_quantity {
                        let new_display = *display_qty - *amount_filled;
                        order.display_quantity = NonZeroDecimal::new(new_display).ok();
                        // Note: display_quantity can become None (zero) for partial fills

                        // invariant: if display_quantity exists, it must be ≤ remaining_quantity
                        if let Some(display_qty_after) = order.display_quantity {
                            assert!(
                                display_qty_after <= order.remaining_quantity,
                                "invariant: display_quantity ({}) cannot exceed remaining_quantity ({}) after partial fill",
                                *display_qty_after,
                                *order.remaining_quantity
                            );
                        }
                    }
                }
                FillType::Cancelled => {
                    // Remove the cancelled order from the book
                    self.orderbook.remove(oix);
                }
                FillType::NotFilled => unreachable!("not possible"),
            }
        }

        self.orderbook
    }
}
