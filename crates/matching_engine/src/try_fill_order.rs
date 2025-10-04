//! This module contains the [`try_fill_orders`] function, which attempts to fill a taker's order

use super::orderbook::Order;
use super::orderbook::OrderSide;
use super::orderbook::OrderType;
use super::orderbook::Orderbook;
use super::pending_fill::FillType;
use super::pending_fill::PendingFill;
use super::*;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::place_order::PlaceOrderDetails;
use crate::self_trade_protection::SelfTradeProtection;
use pending_fill::Fill;
use std::convert::Infallible;

/// An error that can occur when attempting to fill orders.
#[derive(Debug, thiserror::Error)]
pub enum TryFillOrdersError {
    /// quantity is zero
    #[error("quantity is zero")]
    ZeroQuantity,
    /// price is zero
    #[error("price is zero")]
    ZeroPrice,
}

/// Attempts to fill a taker's order against the current state of the order book.
///
/// This function returns a [`PendingFill`] object that encapsulates the potential outcome
/// of the fill operation. This allows you to review the potential outcome before committing
/// to modifying the order book.
///
pub fn try_fill_orders<'a, D>(
    orderbook: &'a mut Orderbook,
    details: &D,
) -> Result<PendingFill<'a>, TryFillOrdersError>
where
    D: PlaceOrderDetails,
{
    let mut maker_fills = vec![];
    let mut taker_fill_outcome = FillType::NotFilled;
    let mut taker_filled_q = crate::decimal::Decimal::ZERO;

    let taker_side = details.order_side();
    let taker_price = details.price().ok_or(TryFillOrdersError::ZeroPrice)?;
    let taker_quantity = details.quantity().ok_or(TryFillOrdersError::ZeroQuantity)?;
    let maker_side = match taker_side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    };

    for (oix, order) in orderbook.iter_rel(maker_side) {
        if details.order_type() == OrderType::Limit
            && ((taker_side == OrderSide::Buy && order.price > taker_price)
                || (taker_side == OrderSide::Sell && order.price < taker_price))
        {
            continue; // Skip orders that don't meet the price condition for limit orders
        }

        assert!(taker_filled_q <= *taker_quantity);

        let fill_amount = std::cmp::min(*order.quantity, (*taker_quantity - taker_filled_q));
        let fill_type = if fill_amount == *order.quantity {
            FillType::Complete
        } else {
            FillType::Partial {
                fill_amount: NonZeroDecimal::new(fill_amount)
                    .expect("fill_amount should be non-zero"),
            }
        };

        maker_fills.push(Fill {
            oix,
            order,
            fill_type,
        });

        if taker_filled_q == fill_amount {
            taker_fill_outcome = FillType::Complete;
            taker_filled_q = *taker_quantity;
            break;
        } else {
            taker_fill_outcome = FillType::Partial;
            taker_filled_q = taker_filled_q + fill_amount;
        }
    }

    // the fill quantity is always between zero and the order quantity
    assert!(taker_filled_q <= *taker_quantity);
    assert!(taker_filled_q >= crate::decimal::Decimal::ZERO);

    if taker_filled_q == *taker_quantity {
        taker_fill_outcome = FillType::Complete;
    } else if taker_filled_q == crate::decimal::Decimal::ZERO {
        taker_fill_outcome = FillType::NotFilled;
    } else {
        taker_fill_outcome = FillType::Partial;
    }

    let pending_fill = PendingFill {
        orderbook,
        fills: maker_fills,
        taker_fill_outcome,
    };

    Ok(pending_fill)
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use crate::orderbook::Order;
    use crate::orderbook::OrderSide;
    use crate::orderbook::OrderType;
    use crate::orderbook::Orderbook;
    use crate::pending_fill::FillType;

    use super::*;

    #[test]
    fn test_exact_match() {
        let mut orderbook = Orderbook::new_empty();
        orderbook.push_ask(Order {
            price: dec!(100).into(),
            quantity: dec!(50).into(),
            memo: 0,
        });

        let taker = Order {
            price: dec!(100).into(),
            quantity: dec!(50).into(),
            memo: 0,
        };
        let result = try_fill_orders(
            &mut orderbook,
            taker,
            OrderSide::Buy,
            OrderType::Limit,
            SelfTradeProtection::CancelBoth,
        )
        .unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::Complete);
        assert_eq!(result.fills.len(), 1);
        assert_eq!(result.fills[0].fill_type, FillType::Complete);
    }

    #[test]
    fn test_partial_fill() {
        let mut orderbook = Orderbook::new_empty();
        orderbook.push_ask(Order {
            price: dec!(100).into(),
            quantity: dec!(30).into(),
            memo: 0,
        });
        let taker = Order {
            price: dec!(100).into(),
            quantity: dec!(50).into(),
            memo: 0,
        };

        let result = try_fill_orders(
            &mut orderbook,
            taker,
            OrderSide::Buy,
            OrderType::Limit,
            SelfTradeProtection::CancelBoth,
        )
        .unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::Partial);
        assert_eq!(result.fills.len(), 1);
        assert_eq!(result.fills[0].fill_type, FillType::Complete);
    }

    #[test]
    fn test_no_possible_fill() {
        let mut orderbook = Orderbook::new_empty();
        orderbook.push_ask(Order {
            price: dec!(150).into(),
            quantity: dec!(50).into(),
            memo: 0,
        });
        let taker = Order {
            price: dec!(100).into(),
            quantity: dec!(50).into(),
            memo: 0,
        };

        let result = try_fill_orders(
            &mut orderbook,
            taker,
            OrderSide::Buy,
            OrderType::Limit,
            SelfTradeProtection::CancelBoth,
        )
        .unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::NotFilled);
        assert_eq!(result.fills.len(), 0);
    }

    #[test]
    fn test_no_matching_orders() {
        let mut orderbook = Orderbook::new_empty();
        let taker = Order {
            price: dec!(100).into(),
            quantity: dec!(50).into(),
            memo: 0,
        };

        let result = try_fill_orders(
            &mut orderbook,
            taker,
            OrderSide::Buy,
            OrderType::Limit,
            SelfTradeProtection::CancelBoth,
        )
        .unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::NotFilled);
        assert_eq!(result.fills.len(), 0);
    }

    #[test]
    fn test_price_mismatch_for_limit_order() {
        let mut orderbook = Orderbook::new_empty();
        orderbook.push_ask(Order {
            price: dec!(150).into(),
            quantity: dec!(50).into(),
            memo: 0,
        });
        let taker = Order {
            price: dec!(100).into(),
            quantity: dec!(50).into(),
            memo: 0,
        };

        let result = try_fill_orders(
            &mut orderbook,
            taker,
            OrderSide::Buy,
            OrderType::Limit,
            SelfTradeProtection::CancelBoth,
        )
        .unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::NotFilled);
        assert_eq!(result.fills.len(), 0);
    }

    #[test]
    fn test_fulfillment_with_multiple_asks() {
        let mut orderbook = Orderbook::new_empty();

        // Adding multiple sell orders at different prices and quantities
        orderbook.push_ask(Order {
            price: dec!(100).into(),
            quantity: dec!(30).into(),
            memo: 1,
        });
        orderbook.push_ask(Order {
            price: dec!(105).into(),
            quantity: dec!(20).into(),
            memo: 2,
        });
        orderbook.push_ask(Order {
            price: dec!(110).into(),
            quantity: dec!(50).into(),
            memo: 3,
        });

        let taker = Order {
            price: dec!(110).into(),   // Taker is willing to buy up to this price
            quantity: dec!(75).into(), // Taker wants a total of 75 units
            memo: 4,
        };

        let result = try_fill_orders(
            &mut orderbook,
            taker,
            OrderSide::Buy,
            OrderType::Limit,
            SelfTradeProtection::CancelBoth,
        )
        .unwrap();

        // Assertions on overall outcome
        assert_eq!(result.taker_fill_outcome, FillType::Complete);
        assert_eq!(
            result.fills.len(),
            3,
            "{maker_fills:#?}",
            maker_fills = result.fills
        );

        // Assertions on individual fills - detailed assertion on each fill type
        assert_eq!(result.fills[0].order.price, dec!(100).into());
        assert_eq!(result.fills[0].fill_type, FillType::Complete);
        assert_eq!(result.fills[0].fill_amount, dec!(30).into());

        assert_eq!(result.fills[1].order.price, dec!(105).into());
        assert_eq!(result.fills[1].fill_type, FillType::Complete);
        assert_eq!(result.fills[1].fill_amount, dec!(20).into());

        assert_eq!(result.fills[2].order.price, dec!(110).into());
        assert_eq!(result.fills[2].fill_type, FillType::Partial); // Correctly marked as Partial
        assert_eq!(result.fills[2].fill_amount, dec!(25).into()); // Only 25 units filled from this order

        // Asserting the exact quantities and conditions met
        let total_filled_quantity = result
            .fills
            .iter()
            .map(|Fill { fill_amount, .. }| **fill_amount)
            .sum::<Decimal>();
        assert_eq!(
            total_filled_quantity,
            dec!(75),
            "Total filled quantity should match the taker's required quantity."
        );

        // Assertions on the PendingFill structure
        assert_eq!(result.taker.price, dec!(110).into());
        assert_eq!(result.taker.quantity, dec!(75).into()); // Ensure original taker's quantity remains unchanged in the struct
        assert_eq!(result.side, OrderSide::Buy);
        assert_eq!(result.order_type, OrderType::Limit);
        assert_eq!(result.taker_fill_outcome, FillType::Complete);
    }
}
