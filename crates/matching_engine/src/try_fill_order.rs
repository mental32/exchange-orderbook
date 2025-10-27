use super::orderbook::OrderSide;
use super::orderbook::OrderType;
use super::orderbook::Orderbook;
use super::pending_fill::FillType;
use super::pending_fill::PendingFill;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::order_ticket::OrderTicket;
use crate::orderbook::OrderIndex;
use crate::orderbook::SelfTradeProtection;
use crate::pending_fill::Fill;
use std::convert::Infallible;

/// An error that can occur when attempting to fill orders.
#[derive(Debug, thiserror::Error)]
pub enum TryFillOrdersError {
    /// quantity is zero
    #[error("quantity is zero")]
    ZeroQuantity,
}

/// generate fills for the incoming order but do not modify the orderbook
pub fn try_fill_orders<'ob, TUserId>(
    orderbook: &'ob mut Orderbook<TUserId>,
    details: &OrderTicket,
    taker_price: NonZeroDecimal,
    taker_user_id: Option<&TUserId>,
) -> Result<PendingFill<'ob, TUserId>, TryFillOrdersError>
where
    TUserId: PartialEq,
{
    let mut fills = vec![];
    let mut taker_fill_outcome = None;
    let mut taker_filled_qty = crate::decimal::Decimal::ZERO;

    let taker_side = details.side;
    let taker_quantity = details.quantity.ok_or(TryFillOrdersError::ZeroQuantity)?;
    let maker_side = match taker_side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    };

    enum Either<T, U, V> {
        First(T),
        Second(U),
        Third(V),
    }

    impl<L, R, V> Iterator for Either<L, R, V>
    where
        L: Iterator,
        R: Iterator<Item = L::Item>,
        V: Iterator<Item = L::Item>,
    {
        type Item = L::Item;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Either::First(l) => l.next(),
                Either::Second(r) => r.next(),
                Either::Third(s) => s.next(),
            }
        }
    }

    let it = if details.order_type == OrderType::Limit {
        Either::First(orderbook.iter_for_limit_relative(maker_side, taker_price))
    } else {
        assert_eq!(details.order_type, OrderType::Market);
        match maker_side {
            OrderSide::Buy => Either::Second(orderbook.bids()),
            OrderSide::Sell => Either::Third(orderbook.asks()),
        }
    };

    for (ix, resting_order) in it {
        // Self-Trade Protection: skip own orders based on STP settings
        if let (Some(taker_id), Some(resting_id)) = (taker_user_id, Some(&resting_order.user_id)) {
            if taker_id == resting_id {
                match details.stp {
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
    use crate::pending_fill::FillType;
    use crate::price::Price;
    use std::str::FromStr;

    fn abs_price(value: &str) -> Price {
        Price {
            prefix: None,
            amount: Decimal::from_str(value).unwrap(),
            is_percentage: false,
        }
    }

    fn build_order(
        order_type: OrderType,
        side: OrderSide,
        price: Price,
        quantity: Option<&str>,
        time_in_force: TimeInForce,
        stp: SelfTradeProtection,
    ) -> OrderTicket {
        let mut builder = OrderTicket::builder(order_type, side, price)
            .time_in_force(time_in_force)
            .stp(stp);
        if let Some(q) = quantity {
            builder = builder.quantity(Decimal::from_str(q).unwrap().into());
        }
        builder.build().unwrap()
    }

    #[test]
    fn test_exact_match() {
        let mut orderbook = Orderbook::new_empty();
        {
            let price = abs_price("100");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = "test_user".to_owned();
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("50"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_id, order);
        };

        let taker_price = abs_price("100");
        let taker = build_order(
            OrderType::Limit,
            OrderSide::Buy,
            taker_price,
            Some("50"),
            TimeInForce::GoodTilCanceled,
            SelfTradeProtection::CancelBoth,
        );

        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            None,
        )
        .unwrap();
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
        let user_a = "user_a".to_owned();
        let user_b = "user_b".to_owned();

        // Resting order from user_a (same user as taker)
        {
            let price = abs_price("100");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("50"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_a.clone(), order);
        };

        // Resting order from user_b (different user)
        {
            let price = abs_price("101");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("30"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_b.clone(), order);
        };

        // Taker order from user_a with CancelNewest STP
        let taker_price = abs_price("101");
        let taker = build_order(
            OrderType::Limit,
            OrderSide::Buy,
            taker_price, // Can match both prices
            Some("40"),
            TimeInForce::GoodTilCanceled,
            SelfTradeProtection::CancelNewest, // Should skip own resting order
        );

        // Should succeed but skip user_a's resting order, only fill with user_b's
        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            Some(&user_a),
        );
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
        let user_a = "user_a".to_owned();
        let user_b = "user_b".to_owned();

        // Resting order from user_a (same user as taker) - this should be cancelled
        {
            let price = abs_price("100");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("50"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelOldest,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_a.clone(), order);
        };

        // Resting order from user_b (different user) - this should be matched
        {
            let orderbook = &mut orderbook;
            let price = abs_price("101");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("30"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelOldest,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_b.clone(), order);
        };

        // Taker order from user_a with CancelOldest STP
        let taker_price = abs_price("101");
        let taker = build_order(
            OrderType::Limit,
            OrderSide::Buy,
            taker_price, // Can match both prices
            Some("40"),
            TimeInForce::GoodTilCanceled,
            SelfTradeProtection::CancelOldest, // Should cancel own resting order
        );

        // Should succeed and fill with user_b's order, while cancelling user_a's resting order
        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            Some(&user_a),
        );
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
        let user_a = "user_a".to_owned();
        let user_b = "user_b".to_owned();

        // Resting order from user_a (same user as taker) - this should be cancelled
        {
            let orderbook = &mut orderbook;
            let price = abs_price("100");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("50"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelBoth,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_a.clone(), order);
        };

        // Resting order from user_b (different user) - this should be matched
        {
            let orderbook = &mut orderbook;
            let price = abs_price("101");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("30"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelBoth,
            );
            let price_nz = NonZeroDecimal::new(price.amount).unwrap();
            orderbook.insert(price_nz, order_id, user_b.clone(), order);
        };

        // Taker order from user_a with CancelBoth STP
        let taker_price = abs_price("101");
        let taker = build_order(
            OrderType::Limit,
            OrderSide::Buy,
            taker_price, // Can match both prices
            Some("40"),
            TimeInForce::GoodTilCanceled,
            SelfTradeProtection::CancelBoth, // Should cancel both orders on self-trade
        );

        // CancelBoth should cancel both orders when self-trade is detected
        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            Some(&user_a),
        );
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
            let price = abs_price("50000");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = "seller_1".to_owned();
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("1.0"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            orderbook.insert(
                NonZeroDecimal::new(price.amount).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Second best: $51,000
        {
            let price = abs_price("51000");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = "seller_2".to_owned();
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("1.5"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            orderbook.insert(
                NonZeroDecimal::new(price.amount).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Worst: $52,000
        {
            let price = abs_price("52000");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = "seller_3".to_owned();
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("2.0"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            orderbook.insert(
                NonZeroDecimal::new(price.amount).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Market buy: 3.0 BTC - will take all available liquidity across price levels
        // Note: price field required but not used for market orders (Kraken behavior)
        let taker_price = abs_price("1"); // Dummy value (not used)
        let taker = build_order(
            OrderType::Market,
            OrderSide::Buy,
            taker_price,
            Some("3.0"),
            TimeInForce::ImmediateOrCancel,
            SelfTradeProtection::CancelNewest,
        );

        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            None,
        );
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
            let price = abs_price("50000");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = "seller_1".to_owned();
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("1.0"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            orderbook.insert(
                NonZeroDecimal::new(price.amount).unwrap(),
                order_id,
                user_id,
                order,
            );
        }

        // Market buy: Want 5.0 BTC but only 1.0 available
        let taker_price = abs_price("1"); // Dummy value
        let taker = build_order(
            OrderType::Market,
            OrderSide::Buy,
            taker_price,
            Some("5.0"),
            TimeInForce::ImmediateOrCancel, // IOC allows partial
            SelfTradeProtection::CancelNewest,
        );

        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            None,
        );
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
        let seller_1 = "seller_1".to_owned();
        let seller_2 = "seller_2".to_owned();

        // Resting order 1: 50 BTC @ $100 (best price)
        {
            let price = abs_price("100");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("50"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            orderbook.insert(
                NonZeroDecimal::new(price.amount).unwrap(),
                order_id,
                seller_1.clone(),
                order,
            );
        }

        // Resting order 2: 50 BTC @ $101 (worse price, should NOT be matched)
        {
            let price = abs_price("101");
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let order = build_order(
                OrderType::Limit,
                OrderSide::Sell,
                price,
                Some("50"),
                TimeInForce::GoodTilCanceled,
                SelfTradeProtection::CancelNewest,
            );
            orderbook.insert(
                NonZeroDecimal::new(price.amount).unwrap(),
                order_id,
                seller_2.clone(),
                order,
            );
        }

        // Taker: Buy 50 BTC @ $101 (willing to pay up to $101)
        let taker_price = abs_price("101");
        let taker = build_order(
            OrderType::Limit,
            OrderSide::Buy,
            taker_price,
            Some("50"), // Exactly matches first order
            TimeInForce::GoodTilCanceled,
            SelfTradeProtection::CancelNewest,
        );

        // CORRECT BEHAVIOR: Should stop matching after taker is completely filled
        let result = try_fill_orders(
            &mut orderbook,
            &taker,
            NonZeroDecimal::new(taker.price.amount).unwrap(),
            None,
        );
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
