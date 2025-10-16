use super::orderbook::OrderSide;
use super::orderbook::OrderType;
use super::orderbook::Orderbook;
use super::pending_fill::FillType;
use super::pending_fill::PendingFill;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::orderbook::OrderIndex;
use crate::orderbook::SelfTradeProtection;
use crate::pending_fill::Fill;
use crate::pending_fill::OrderDetails;
use common_core::web::middleware::clerk::ClerkUserId;
use std::convert::Infallible;

/// An error that can occur when attempting to fill orders.
#[derive(Debug, thiserror::Error)]
pub enum TryFillOrdersError {
    /// quantity is zero
    #[error("quantity is zero")]
    ZeroQuantity,
}

/// generate fills for the incoming order but do not modify the orderbook
pub fn try_fill_orders<'ob, D>(
    orderbook: &'ob mut Orderbook,
    details: &D,
    taker_price: NonZeroDecimal,
    taker_user_id: Option<&ClerkUserId>,
) -> Result<PendingFill<'ob>, TryFillOrdersError>
where
    D: OrderDetails,
{
    let mut fills = vec![];
    let mut taker_fill_outcome = None;
    let mut taker_filled_qty = crate::decimal::Decimal::ZERO;

    let taker_side = details.order_side();
    let taker_quantity = details.quantity().ok_or(TryFillOrdersError::ZeroQuantity)?;
    let maker_side = match taker_side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    };

    enum Either<L, R> {
        Left(L),
        Right(R),
    }

    impl<L, R> Iterator for Either<L, R>
    where
        L: Iterator,
        R: Iterator<Item = L::Item>,
    {
        type Item = L::Item;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Either::Left(l) => l.next(),
                Either::Right(r) => r.next(),
            }
        }
    }

    let it = if details.order_type() == OrderType::Limit {
        Either::Left(orderbook.iter_rel(maker_side, taker_price))
    } else {
        assert_eq!(details.order_type(), OrderType::Market);
        Either::Right(match maker_side {
            OrderSide::Buy => orderbook.bids(),
            OrderSide::Sell => orderbook.asks(),
        })
    };

    for (ix, resting_order) in it {
        // Self-Trade Protection: skip own orders based on STP settings
        if let (Some(taker_id), Some(resting_id)) = (taker_user_id, Some(&resting_order.user_id)) {
            if taker_id == resting_id {
                match details.stp() {
                    SelfTradeProtection::CancelNewest => {
                        // Cancel the taker order by setting outcome to Cancelled
                        taker_fill_outcome = Some(FillType::Cancelled);
                        break;
                    }
                    SelfTradeProtection::CancelOldest => {
                        // Add cancellation fill for the resting order
                        let oix = OrderIndex {
                            side: maker_side,
                            price: resting_order.price,
                            timestamp: resting_order.timestamp,
                        };
                        fills.push(Fill {
                            order_index: oix,
                            fill_type: FillType::Cancelled,
                        });
                        continue;
                    }
                    SelfTradeProtection::CancelBoth => {
                        // Add cancellation fill for the resting order
                        let oix = OrderIndex {
                            side: maker_side,
                            price: resting_order.price,
                            timestamp: resting_order.timestamp,
                        };
                        fills.push(Fill {
                            order_index: oix,
                            fill_type: FillType::Cancelled,
                        });
                        // Cancel the taker order by setting outcome to Cancelled
                        taker_fill_outcome = Some(FillType::Cancelled);
                        // Stop processing - both orders are cancelled, no trade should execute
                        break;
                    }
                }
            }
        }

        assert!(taker_filled_qty <= *taker_quantity);

        let taker_remaining_qty = *taker_quantity - taker_filled_qty;

        // For Iceberg orders, match against display_quantity; otherwise use remaining_quantity
        let visible_quantity = resting_order
            .display_quantity
            .unwrap_or(resting_order.remaining_quantity);

        let fill_type = match taker_remaining_qty.cmp(&*visible_quantity) {
            std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => FillType::Complete {
                quantity: visible_quantity,
            },
            std::cmp::Ordering::Less => FillType::Partial {
                amount_filled: NonZeroDecimal::new(taker_remaining_qty)
                    .expect("taker_remaining_qty should be non-zero"),
            },
        };

        let order_index = OrderIndex {
            side: maker_side,
            price: resting_order.price,
            timestamp: resting_order.timestamp,
        };

        fills.push(Fill {
            order_index,
            fill_type,
        });

        // Update taker filled quantity based on what was filled
        match fill_type {
            FillType::Complete { quantity } => {
                // Resting order's visible portion was completely filled
                taker_filled_qty += *quantity;
            }
            FillType::Partial { amount_filled } => {
                // Resting order was partially filled, which means taker is now complete
                taker_filled_qty += *amount_filled;
                taker_fill_outcome = Some(FillType::Complete {
                    quantity: taker_filled_qty.into(),
                });
                break;
            }
            FillType::Cancelled => {
                // Cancelled orders don't contribute to taker fill quantity
                // This shouldn't happen in the normal fill matching logic
                unreachable!(
                    "cancelled fills should not be processed in taker quantity calculation"
                );
            }
        }

        // After adding this order's quantity, update taker's status
        if taker_filled_qty == *taker_quantity {
            taker_fill_outcome = Some(FillType::Complete {
                quantity: taker_filled_qty.into(),
            });
            break;
        } else {
            taker_fill_outcome = Some(FillType::Partial {
                amount_filled: NonZeroDecimal::new(taker_filled_qty)
                    .expect("taker_filled_qty should be non-zero at this point"),
            });
        }
    }

    // the fill quantity is always between zero and the order quantity
    assert!(taker_filled_qty <= *taker_quantity);
    assert!(taker_filled_qty >= crate::decimal::Decimal::ZERO);

    let pending_fill = PendingFill {
        orderbook,
        fills,
        taker_fill_outcome,
    };

    Ok(pending_fill)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decimal::Decimal;
    use crate::decimal::NonZeroDecimal;
    use crate::decimal::dec;
    use crate::order_uuid::OrderUuid;
    use crate::orderbook::OrderSide;
    use crate::orderbook::OrderType;
    use crate::orderbook::Orderbook;
    use crate::orderbook::SelfTradeProtection;
    use crate::orderbook::TimeInForce;
    use crate::orderflags::OrderFlags;
    use crate::pending_fill::FillType;
    use crate::pending_fill::OrderDetails;
    use crate::price::Price;
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
        display_quantity: Option<Decimal>,
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
            self.display_quantity
                .and_then(|d| NonZeroDecimal::new(d).ok())
        }

        fn order_flags(&self) -> crate::orderflags::OrderFlags {
            OrderFlags::default()
        }

        fn userref(&self) -> Option<u32> {
            None
        }
    }

    #[test]
    fn test_exact_match() {
        let mut orderbook = Orderbook::new_empty();
        {
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("50").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("test_user".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_id, order);
        };

        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("100").unwrap(),
            quantity: Decimal::from_str("50").unwrap(),
            time_in_force: TimeInForce::GoodTilCanceled,
            stp: SelfTradeProtection::CancelBoth,
            display_quantity: None,
        };

        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), None).unwrap();
        assert_eq!(
            result.taker_fill_outcome,
            Some(FillType::Complete {
                quantity: crate::decimal::dec!(50).into()
            })
        );
        assert_eq!(result.fills.len(), 1);
        assert_eq!(
            result.fills[0].fill_type,
            FillType::Complete {
                quantity: crate::decimal::dec!(50).into()
            }
        );
    }

    #[test]
    fn test_stp_cancel_newest_cancels_taker() {
        let mut orderbook = Orderbook::new_empty();

        // Add resting sell orders: one from same user, one from different user
        let user_a = common_core::web::middleware::clerk::ClerkUserId("user_a".to_string());
        let user_b = common_core::web::middleware::clerk::ClerkUserId("user_b".to_string());

        // Resting order from user_a (same user as taker)
        {
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("50").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_a.clone(), order);
        };

        // Resting order from user_b (different user)
        {
            let price = Decimal::from_str("101").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_b.clone(), order);
        };

        // Taker order from user_a with CancelNewest STP
        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("101").unwrap(), // Can match both prices
            quantity: Decimal::from_str("40").unwrap(),
            time_in_force: TimeInForce::GoodTilCanceled,
            stp: SelfTradeProtection::CancelNewest, // Should skip own resting order
            display_quantity: None,
        };

        // Should succeed but skip user_a's resting order, only fill with user_b's
        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), Some(&user_a));
        assert!(result.is_ok());

        let pending_fill = result.unwrap();
        // Should only have 1 fill (with user_b's order), not 2
        assert_eq!(pending_fill.fills.len(), 0);
        // Taker should be partially filled (30 out of 40)
        assert_eq!(pending_fill.taker_fill_outcome, Some(FillType::Cancelled));
    }

    #[test]
    fn test_stp_cancel_oldest_cancels_resting_order() {
        let mut orderbook = Orderbook::new_empty();

        // Add resting sell orders: one from same user, one from different user
        let user_a = common_core::web::middleware::clerk::ClerkUserId("user_a".to_string());
        let user_b = common_core::web::middleware::clerk::ClerkUserId("user_b".to_string());

        // Resting order from user_a (same user as taker) - this should be cancelled
        {
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("50").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelOldest,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_a.clone(), order);
        };

        // Resting order from user_b (different user) - this should be matched
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("101").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelOldest,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_b.clone(), order);
        };

        // Taker order from user_a with CancelOldest STP
        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("101").unwrap(), // Can match both prices
            quantity: Decimal::from_str("40").unwrap(),
            time_in_force: TimeInForce::GoodTilCanceled,
            stp: SelfTradeProtection::CancelOldest, // Should cancel own resting order
            display_quantity: None,
        };

        // Should succeed and fill with user_b's order, while cancelling user_a's resting order
        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), Some(&user_a));
        assert!(result.is_ok());

        let pending_fill = result.unwrap();
        // Should have 2 fills: 1 cancellation of user_a's order + 1 trade with user_b's order
        assert_eq!(pending_fill.fills.len(), 2);
        // Taker should be partially filled (30 out of 40)
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Partial { .. })
        ));

        // Verify we have both a cancellation and a complete fill
        let cancellation_count = pending_fill
            .fills
            .iter()
            .filter(|f| matches!(f.fill_type, FillType::Cancelled))
            .count();
        let complete_fill_count = pending_fill
            .fills
            .iter()
            .filter(|f| matches!(f.fill_type, FillType::Complete { .. }))
            .count();

        assert_eq!(cancellation_count, 1, "Should have 1 cancellation fill");
        assert_eq!(complete_fill_count, 1, "Should have 1 complete fill");

        // The complete fill should be with user_b's order at price 101
        let complete_fill = pending_fill
            .fills
            .iter()
            .find(|f| matches!(f.fill_type, FillType::Complete { .. }))
            .expect("Should have a complete fill");
        assert_eq!(
            complete_fill.order_index.price,
            NonZeroDecimal::new(dec!(101)).unwrap()
        );
    }

    #[test]
    fn test_stp_cancel_both_cancels_both_orders() {
        let mut orderbook = Orderbook::new_empty();

        // Add resting sell orders: one from same user, one from different user
        let user_a = common_core::web::middleware::clerk::ClerkUserId("user_a".to_string());
        let user_b = common_core::web::middleware::clerk::ClerkUserId("user_b".to_string());

        // Resting order from user_a (same user as taker) - this should be cancelled
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("50").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelBoth,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_a.clone(), order);
        };

        // Resting order from user_b (different user) - this should be matched
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("101").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelBoth,
                display_quantity: None,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_b.clone(), order);
        };

        // Taker order from user_a with CancelBoth STP
        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("101").unwrap(), // Can match both prices
            quantity: Decimal::from_str("40").unwrap(),
            time_in_force: TimeInForce::GoodTilCanceled,
            stp: SelfTradeProtection::CancelBoth, // Should cancel both orders on self-trade
            display_quantity: None,
        };

        // CancelBoth should cancel both orders when self-trade is detected
        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), Some(&user_a));
        assert!(result.is_ok());

        let pending_fill = result.unwrap();
        // Should have 1 fill: only the cancellation of user_a's resting order
        assert_eq!(pending_fill.fills.len(), 1);
        // Taker should be cancelled due to self-trade
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Cancelled)
        ));

        // Verify we have only a cancellation fill
        let cancellation_count = pending_fill
            .fills
            .iter()
            .filter(|f| matches!(f.fill_type, FillType::Cancelled))
            .count();
        let complete_fill_count = pending_fill
            .fills
            .iter()
            .filter(|f| matches!(f.fill_type, FillType::Complete { .. }))
            .count();

        assert_eq!(cancellation_count, 1, "Should have 1 cancellation fill");
        assert_eq!(complete_fill_count, 0, "Should have 0 complete fills");

        // The cancellation fill should be user_a's resting order at price 100
        let cancellation_fill = pending_fill
            .fills
            .iter()
            .find(|f| matches!(f.fill_type, FillType::Cancelled))
            .expect("Should have a cancellation fill");
        assert_eq!(
            cancellation_fill.order_index.price,
            NonZeroDecimal::new(dec!(100)).unwrap()
        );
    }

    #[test]
    fn test_market_buy_crosses_multiple_price_levels() {
        let mut orderbook = Orderbook::new_empty();

        // Setup: Three sell orders at different prices
        // Best ask: $50,000
        {
            let price = Decimal::from_str("50000").unwrap();
            let quantity = Decimal::from_str("1.0").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("seller_1".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            orderbook.insert(
                NonZeroDecimal::new(price).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Second best: $51,000
        {
            let price = Decimal::from_str("51000").unwrap();
            let quantity = Decimal::from_str("1.5").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("seller_2".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            orderbook.insert(
                NonZeroDecimal::new(price).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Worst: $52,000
        {
            let price = Decimal::from_str("52000").unwrap();
            let quantity = Decimal::from_str("2.0").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("seller_3".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            orderbook.insert(
                NonZeroDecimal::new(price).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Market buy: 3.0 BTC - will take all available liquidity across price levels
        // Note: price field required but not used for market orders (Kraken behavior)
        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: Decimal::from_str("1").unwrap(), // Dummy value (not used)
            quantity: Decimal::from_str("3.0").unwrap(),
            time_in_force: TimeInForce::ImmediateOrCancel,
            stp: SelfTradeProtection::CancelNewest,
            display_quantity: None,
        };

        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), None);
        assert!(result.is_ok());

        let pending_fill = result.unwrap();

        // Should have 3 fills (one from each seller)
        assert_eq!(pending_fill.fills.len(), 3);

        // Taker should be fully filled
        assert_eq!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Complete {
                quantity: crate::decimal::dec!(3.0).into()
            })
        );

        // Verify fill order and prices (best price first)
        // Fill 1: 1.0 BTC @ $50k (complete)
        assert_eq!(
            pending_fill.fills[0].order_index.price,
            NonZeroDecimal::new(dec!(50000)).unwrap()
        );
        assert_eq!(
            pending_fill.fills[0].fill_type,
            FillType::Complete {
                quantity: crate::decimal::dec!(1.0).into()
            }
        );

        // Fill 2: 1.5 BTC @ $51k (complete)
        assert_eq!(
            pending_fill.fills[1].order_index.price,
            NonZeroDecimal::new(dec!(51000)).unwrap()
        );
        assert_eq!(
            pending_fill.fills[1].fill_type,
            FillType::Complete {
                quantity: crate::decimal::dec!(1.5).into()
            }
        );

        // Fill 3: 0.5 BTC @ $52k (partial - only need 0.5 to reach 3.0 total)
        assert_eq!(
            pending_fill.fills[2].order_index.price,
            NonZeroDecimal::new(dec!(52000)).unwrap()
        );
        assert!(matches!(
            pending_fill.fills[2].fill_type,
            FillType::Partial { amount_filled } if *amount_filled == dec!(0.5)
        ));

        // Total cost calculation (for buy-side reservation logic):
        // 1.0 × $50k + 1.5 × $51k + 0.5 × $52k = $50k + $76.5k + $26k = $152,500
    }

    #[test]
    fn test_market_buy_with_insufficient_liquidity() {
        let mut orderbook = Orderbook::new_empty();

        // Only 1 BTC available at $50k
        {
            let price = Decimal::from_str("50000").unwrap();
            let quantity = Decimal::from_str("1.0").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("seller_1".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            orderbook.insert(
                NonZeroDecimal::new(price).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Market buy: Want 5.0 BTC but only 1.0 available
        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: Decimal::from_str("1").unwrap(), // Dummy value
            quantity: Decimal::from_str("5.0").unwrap(),
            time_in_force: TimeInForce::ImmediateOrCancel, // IOC allows partial
            stp: SelfTradeProtection::CancelNewest,
            display_quantity: None,
        };

        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), None);
        assert!(result.is_ok());

        let pending_fill = result.unwrap();

        // Should have 1 fill
        assert_eq!(pending_fill.fills.len(), 1);

        // Taker should be PARTIALLY filled (only got 1.0 out of 5.0)
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Partial { amount_filled }) if *amount_filled == dec!(1.0)
        ));
    }

    #[test]
    fn test_taker_complete_stops_matching() {
        let mut orderbook = Orderbook::new_empty();

        // Setup: Two sell orders, taker will be completely filled by first one
        let seller_1 = common_core::web::middleware::clerk::ClerkUserId("seller_1".to_string());
        let seller_2 = common_core::web::middleware::clerk::ClerkUserId("seller_2".to_string());

        // Resting order 1: 50 BTC @ $100 (best price)
        {
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("50").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            orderbook.insert(
                NonZeroDecimal::new(price).unwrap(),
                order_id,
                seller_1.clone(),
                order,
            );
        }

        // Resting order 2: 50 BTC @ $101 (worse price, should NOT be matched)
        {
            let price = Decimal::from_str("101").unwrap();
            let quantity = Decimal::from_str("50").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
                display_quantity: None,
            };
            orderbook.insert(
                NonZeroDecimal::new(price).unwrap(),
                order_id,
                seller_2.clone(),
                order,
            );
        }

        // Taker: Buy 50 BTC @ $101 (willing to pay up to $101)
        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("101").unwrap(),
            quantity: Decimal::from_str("50").unwrap(), // Exactly matches first order
            time_in_force: TimeInForce::GoodTilCanceled,
            stp: SelfTradeProtection::CancelNewest,
            display_quantity: None,
        };

        // CORRECT BEHAVIOR: Should stop matching after taker is completely filled
        let result = try_fill_orders(&mut orderbook, &taker, taker.price.into(), None);
        assert!(result.is_ok());

        let pending_fill = result.unwrap();

        // Should only match with first order (50 BTC @ $100)
        assert_eq!(
            pending_fill.fills.len(),
            1,
            "Should stop after taker is complete"
        );

        // Taker should be completely filled
        assert_eq!(
            pending_fill.taker_fill_outcome,
            Some(FillType::Complete {
                quantity: crate::decimal::dec!(50).into()
            })
        );

        // The single fill should be at $100 (first order)
        assert_eq!(
            pending_fill.fills[0].order_index.price,
            NonZeroDecimal::new(dec!(100)).unwrap()
        );
        assert_eq!(
            pending_fill.fills[0].fill_type,
            FillType::Complete {
                quantity: crate::decimal::dec!(50).into()
            }
        );
    }
}
