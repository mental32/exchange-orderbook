//! Pending fill operations on the [`Orderbook`].

use super::asset_code::AssetCode;
use super::orderbook::OrderData;
use super::orderbook::OrderIndex;
use super::orderbook::OrderSide;
use super::orderbook::OrderType;
use super::orderbook::Orderbook;
use super::orderbook::TimeInForce;
use super::try_fill_order::try_fill_orders;
use crate::asset_pair::BaseQuote;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::SelfTradeProtection;
use crate::orderflags::OrderFlags;
use crate::price::Price;
use crate::reserve_money::ReserveByAssetError;
use common_core::web::middleware::clerk::ClerkUserId;

pub trait OrderDetails: Clone {
    fn order_side(&self) -> OrderSide;
    fn quantity(&self) -> Option<NonZeroDecimal>;
    fn price(&self) -> Price;
    fn order_type(&self) -> OrderType;
    fn time_in_force(&self) -> TimeInForce;
    fn stp(&self) -> SelfTradeProtection;
    /// For Iceberg orders, returns the display quantity (visible in the book) should return None for non-iceberg orders.
    fn display_quantity(&self) -> Option<NonZeroDecimal>;
    fn order_flags(&self) -> OrderFlags;
    /// User-specified reference number that can be associated with orders
    fn userref(&self) -> Option<u32>;
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MatchEvent {
    Trade {
        aggregate_filled_quantity: NonZeroDecimal,
        taker_order_id: OrderUuid,
        // taker_user_id: ClerkUserId,
        order_side: OrderSide,
        taker_fully_filled: bool,
        // timestamp: (),
        // trades: Vec<OrderIndex>,
    },
    Reduce {
        amount_reduced: NonZeroDecimal,
        order_removed: bool,
        order_price: NonZeroDecimal,
        order_id: OrderUuid,
        // user_id: ClerkUserId,
        // timestamp: ()
    },
    Reject {
        amount_rejected: NonZeroDecimal,
        #[cfg_attr(feature = "serde", serde(with = "crate::price"))]
        price: Price,
        order_id: OrderUuid,
        // user_id: ClerkUserId,
        // timestamp: ()
    },
}

/// The outcome of a fill operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FillType {
    /// The order was completely filled.
    Complete { quantity: NonZeroDecimal },
    /// The order was partially filled.
    Partial { amount_filled: NonZeroDecimal },
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
    pub taker_fill_outcome: Option<FillType>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum TifViolation {
    /// FillOrKill order was not completely filled (got partial or no fill)
    #[error("FillOrKill order was not completely filled")]
    FillOrKill,
    /// ImmediateOrCancel order could not fill any quantity
    #[error("ImmediateOrCancel order had no liquidity available")]
    ImmediateOrCancel,
}

impl<'a> PendingFill<'a> {
    pub fn has_tif_violation(
        &self,
        order_type: OrderType,
        time_in_force: TimeInForce,
    ) -> Result<(), TifViolation> {
        use OrderType as T;

        let taker_fill_outcome = self.taker_fill_outcome.as_ref();

        let tif_violation: Option<TifViolation> =
            match (order_type, time_in_force, taker_fill_outcome) {
                (T::Limit, TimeInForce::GoodTilCanceled, None) => None,
                (T::Limit, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::Limit, TimeInForce::GoodTilDate(_date), None) => None,
                (T::Limit, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::Limit, TimeInForce::ImmediateOrCancel, None) => {
                    Some(TifViolation::ImmediateOrCancel)
                }
                (T::Limit, TimeInForce::ImmediateOrCancel, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } | FillType::Partial { .. } => None,
                    FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                },
                (T::Limit, TimeInForce::FillOrKill, None) => Some(TifViolation::FillOrKill),
                (T::Limit, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::Market, TimeInForce::GoodTilCanceled, None) => {
                    Some(TifViolation::ImmediateOrCancel)
                }
                (T::Market, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::Market, TimeInForce::GoodTilDate(_date), None) => {
                    Some(TifViolation::ImmediateOrCancel)
                }
                (T::Market, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::Market, TimeInForce::ImmediateOrCancel, None) => {
                    Some(TifViolation::ImmediateOrCancel)
                }
                (T::Market, TimeInForce::ImmediateOrCancel, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } | FillType::Partial { .. } => None,
                    FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                },
                (T::Market, TimeInForce::FillOrKill, None) => Some(TifViolation::FillOrKill),
                (T::Market, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::StopLoss, TimeInForce::GoodTilCanceled, None) => None,
                (T::StopLoss, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::StopLoss, TimeInForce::GoodTilDate(_date), None) => None,
                (T::StopLoss, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::StopLoss, TimeInForce::ImmediateOrCancel, None) => None,
                (T::StopLoss, TimeInForce::ImmediateOrCancel, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } | FillType::Partial { .. } => None,
                    FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                },
                (T::StopLoss, TimeInForce::FillOrKill, None) => None,
                (T::StopLoss, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::StopLossLimit, TimeInForce::GoodTilCanceled, None) => None,
                (T::StopLossLimit, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::StopLossLimit, TimeInForce::GoodTilDate(_date), None) => None,
                (T::StopLossLimit, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::StopLossLimit, TimeInForce::ImmediateOrCancel, None) => None,
                (T::StopLossLimit, TimeInForce::ImmediateOrCancel, Some(fill_type)) => {
                    match fill_type {
                        FillType::Complete { .. } | FillType::Partial { .. } => None,
                        FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                    }
                }
                (T::StopLossLimit, TimeInForce::FillOrKill, None) => None,
                (T::StopLossLimit, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::TakeProfit, TimeInForce::FillOrKill, None) => None,
                (T::TakeProfit, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::TakeProfit, TimeInForce::GoodTilCanceled, None) => None,
                (T::TakeProfit, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::TakeProfit, TimeInForce::GoodTilDate(_date), None) => None,
                (T::TakeProfit, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::TakeProfit, TimeInForce::ImmediateOrCancel, None) => None,
                (T::TakeProfit, TimeInForce::ImmediateOrCancel, Some(fill_type)) => match fill_type
                {
                    FillType::Complete { .. } | FillType::Partial { .. } => None,
                    FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                },
                (T::TakeProfit, TimeInForce::FillOrKill, None) => None,
                (T::TakeProfit, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::TakeProfitLimit, TimeInForce::GoodTilCanceled, None) => None,
                (T::TakeProfitLimit, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::TakeProfitLimit, TimeInForce::GoodTilDate(_date), None) => None,
                (T::TakeProfitLimit, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::TakeProfitLimit, TimeInForce::ImmediateOrCancel, None) => None,
                (T::TakeProfitLimit, TimeInForce::ImmediateOrCancel, Some(fill_type)) => {
                    match fill_type {
                        FillType::Complete { .. } | FillType::Partial { .. } => None,
                        FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                    }
                }
                (T::TakeProfitLimit, TimeInForce::FillOrKill, None) => None,
                (T::TakeProfitLimit, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::TrailingStop, TimeInForce::GoodTilCanceled, None) => None,
                (T::TrailingStop, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::TrailingStop, TimeInForce::GoodTilDate(_date), None) => None,
                (T::TrailingStop, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::TrailingStop, TimeInForce::ImmediateOrCancel, None) => None,
                (T::TrailingStop, TimeInForce::ImmediateOrCancel, Some(fill_type)) => {
                    match fill_type {
                        FillType::Complete { .. } | FillType::Partial { .. } => None,
                        FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                    }
                }
                (T::TrailingStop, TimeInForce::FillOrKill, None) => None,
                (T::TrailingStop, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::TrailingStopLimit, TimeInForce::GoodTilCanceled, None) => None,
                (T::TrailingStopLimit, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::TrailingStopLimit, TimeInForce::GoodTilDate(_date), None) => None,
                (T::TrailingStopLimit, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::TrailingStopLimit, TimeInForce::ImmediateOrCancel, None) => None,
                (T::TrailingStopLimit, TimeInForce::ImmediateOrCancel, Some(fill_type)) => {
                    match fill_type {
                        FillType::Complete { .. } | FillType::Partial { .. } => None,
                        FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                    }
                }
                (T::TrailingStopLimit, TimeInForce::FillOrKill, None) => None,
                (T::TrailingStopLimit, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type
                {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
                (T::Iceberg, TimeInForce::GoodTilCanceled, None) => None,
                (T::Iceberg, TimeInForce::GoodTilCanceled, Some(_)) => None,
                (T::Iceberg, TimeInForce::GoodTilDate(_date), None) => None,
                (T::Iceberg, TimeInForce::GoodTilDate(_date), Some(_)) => None,
                (T::Iceberg, TimeInForce::ImmediateOrCancel, None) => {
                    Some(TifViolation::ImmediateOrCancel)
                }
                (T::Iceberg, TimeInForce::ImmediateOrCancel, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } | FillType::Partial { .. } => None,
                    FillType::Cancelled => Some(TifViolation::ImmediateOrCancel),
                },
                (T::Iceberg, TimeInForce::FillOrKill, None) => Some(TifViolation::FillOrKill),
                (T::Iceberg, TimeInForce::FillOrKill, Some(fill_type)) => match fill_type {
                    FillType::Complete { .. } => None,
                    FillType::Partial { .. } | FillType::Cancelled => {
                        Some(TifViolation::FillOrKill)
                    }
                },
            };

        match tif_violation {
            Some(t) => Err(t.into()),
            None => Ok(()),
        }
    }

    pub fn filled_qty(&self) -> crate::decimal::Decimal {
        self.fills
            .iter()
            .map(|fill| match fill.fill_type {
                FillType::Complete { quantity } => *quantity,
                FillType::Partial { amount_filled } => *amount_filled,
                FillType::Cancelled => crate::decimal::Decimal::ZERO,
            })
            .sum()
    }

    pub fn match_events<D>(&self, order_uuid: OrderUuid, details: D) -> Vec<MatchEvent>
    where
        D: OrderDetails,
    {
        let mut events: Vec<MatchEvent> = self
            .fills
            .iter()
            .map(|fill| {
                let order_data = self.orderbook.get(fill.order_index).expect("always exists");
                let amount_filled = if let FillType::Partial { amount_filled } = fill.fill_type {
                    amount_filled
                } else if let FillType::Complete { quantity } = fill.fill_type {
                    quantity
                } else {
                    // Cancelled fills - use zero amount since no trade occurred
                    crate::decimal::NonZeroDecimal::new(crate::decimal::Decimal::ONE).unwrap()
                };

                MatchEvent::Reduce {
                    amount_reduced: amount_filled,
                    order_removed: matches!(fill.fill_type, FillType::Complete { .. }),
                    order_price: fill.order_index.price,
                    order_id: order_data.order_id,
                }
            })
            .collect();

        if let Some(fill_type) = self.taker_fill_outcome {
            match fill_type {
                FillType::Complete { quantity } => {
                    events.push(MatchEvent::Trade {
                        aggregate_filled_quantity: quantity,
                        taker_order_id: order_uuid,
                        order_side: details.order_side(),
                        taker_fully_filled: true,
                        // trades: pending_fill.fills,
                    });
                }
                FillType::Partial { amount_filled } => {
                    events.push(MatchEvent::Trade {
                        aggregate_filled_quantity: amount_filled,
                        taker_order_id: order_uuid,
                        order_side: details.order_side(),
                        taker_fully_filled: false,
                        // trades: pending_fill.fills,
                    });
                }
                FillType::Cancelled => events.push(MatchEvent::Reject {
                    amount_rejected: details.quantity().unwrap(),
                    price: details.price(),
                    order_id: order_uuid,
                }),
            }
        };

        events
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
                FillType::Complete { .. } => {
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
            }
        }

        self.orderbook
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_code::SymbolVocabulary;
    use crate::decimal::Decimal;
    use crate::order_uuid::OrderUuid;
    use crate::orderbook::Orderbook;
    use crate::try_fill_order::try_fill_orders;
    use std::str::FromStr;

    // Test helper struct that implements OrderDetails
    #[derive(Clone)]
    struct TestOrder {
        side: OrderSide,
        order_type: OrderType,
        price: Decimal,
        quantity: Decimal,
        time_in_force: TimeInForce,
        stp: SelfTradeProtection,
    }

    impl OrderDetails for TestOrder {
        fn order_side(&self) -> OrderSide {
            self.side
        }

        fn quantity(&self) -> Option<NonZeroDecimal> {
            NonZeroDecimal::new(self.quantity).ok()
        }

        fn price(&self) -> Price {
            Price {
                prefix: None,
                amount: self.price,
                is_percentage: false,
            }
        }

        fn order_type(&self) -> OrderType {
            self.order_type
        }

        fn time_in_force(&self) -> TimeInForce {
            self.time_in_force
        }

        fn stp(&self) -> SelfTradeProtection {
            self.stp
        }

        fn display_quantity(&self) -> Option<NonZeroDecimal> {
            None
        }

        fn order_flags(&self) -> OrderFlags {
            OrderFlags::default()
        }

        fn userref(&self) -> Option<u32> {
            None
        }
    }

    #[test]
    fn test_ioc_partial_fill_succeeds() {
        let mut orderbook = Orderbook::new_empty();

        // Only 30 units available, but taker wants 50
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("test_user".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_id, order);
        };

        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("100").unwrap(),
            quantity: Decimal::from_str("50").unwrap(),
            time_in_force: TimeInForce::ImmediateOrCancel, // IOC allows partial fills
            stp: SelfTradeProtection::CancelBoth,
        };

        let pending_fill =
            try_fill_orders(&mut orderbook, &taker, taker.price.into(), None).unwrap();

        // Verify it's a partial fill
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Partial { .. })
        ));

        // Create vocabulary and asset codes properly
        let vocab: SymbolVocabulary = vec!["BTC".to_string(), "USD".to_string()]
            .into_iter()
            .collect();
        let btc = AssetCode::from_str_and_vocabulary("BTC", &vocab).unwrap();
        let usd = AssetCode::from_str_and_vocabulary("USD", &vocab).unwrap();

        // IOC should succeed with partial fill
        assert_eq!(
            pending_fill.has_tif_violation(taker.order_type, taker.time_in_force),
            Ok(())
        );
    }

    #[test]
    fn test_fok_fails_on_partial_fill() {
        let mut orderbook = Orderbook::new_empty();

        // Only 30 units available, but taker wants 50
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("test_user".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_id, order);
        };

        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("100").unwrap(),
            quantity: Decimal::from_str("50").unwrap(),
            time_in_force: TimeInForce::FillOrKill, // FOK requires complete fill
            stp: SelfTradeProtection::CancelBoth,
        };

        let pending_fill =
            try_fill_orders(&mut orderbook, &taker, taker.price.into(), None).unwrap();

        // Verify it's a partial fill
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Partial { .. })
        ));

        // Create vocabulary and asset codes properly
        let vocab: SymbolVocabulary = vec!["BTC".to_string(), "USD".to_string()]
            .into_iter()
            .collect();
        let btc = AssetCode::from_str_and_vocabulary("BTC", &vocab).unwrap();
        let usd = AssetCode::from_str_and_vocabulary("USD", &vocab).unwrap();

        // FOK should fail on partial fill with TifViolation error
        assert!(matches!(
            pending_fill.has_tif_violation(taker.order_type, taker.time_in_force),
            Err(TifViolation::FillOrKill)
        ));
    }
}
