//! This module contains the [`try_fill_orders`] function, which attempts to fill a taker's order

use std::convert::Infallible;

use pending_fill::MakerFill;

use super::*;

/// An error that can occur when attempting to fill orders.
#[derive(Debug, thiserror::Error)]
pub enum TryFillOrdersError {}

/// Attempts to fill a taker's order against the current state of the order book.
///
/// This function returns a [`PendingFill`] object that encapsulates the potential outcome
/// of the fill operation. This allows you to review the potential outcome before committing
/// to modifying the order book.
///
pub fn try_fill_orders<'a>(
    orderbook: &'a mut Orderbook,
    taker: Order,
    side: OrderSide,
    order_type: OrderType,
) -> Result<PendingFill<'a>, Infallible> {
    let mut maker_fills = vec![];
    let mut taker_fill_outcome = FillType::None;
    let mut taker_rem_q = taker.quantity.get();

    let maker_side = match side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    };

    for (oix, order) in orderbook.iter_rel(maker_side) {
        if order_type == OrderType::Limit
            && ((side == OrderSide::Buy && order.price > taker.price)
                || (side == OrderSide::Sell && order.price < taker.price))
        {
            continue; // Skip orders that don't meet the price condition for limit orders
        }

        let fill_amount = std::cmp::min(order.quantity.get(), taker_rem_q);
        let fill_type = if fill_amount == order.quantity.get() {
            FillType::Complete
        } else {
            FillType::Partial
        };

        maker_fills.push(MakerFill {
            oix,
            maker: order,
            fill_type,
            fill_amount,
        });

        if taker_rem_q == fill_amount {
            taker_fill_outcome = FillType::Complete;
            taker_rem_q = 0;
            break;
        } else {
            taker_fill_outcome = FillType::Partial;
            taker_rem_q = taker_rem_q - fill_amount;
        }
    }

    if taker_rem_q == taker.quantity.get() {
        taker_fill_outcome = FillType::None;
    }

    let pending_fill = PendingFill::new(
        orderbook,
        taker,
        side,
        order_type,
        maker_fills,
        taker_fill_outcome,
    );

    Ok(pending_fill)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! nz {
        ($e:literal) => {
            ::std::num::NonZeroU32::new($e).unwrap()
        };
    }

    #[test]
    fn test_exact_match() {
        let mut orderbook = Orderbook::new();
        orderbook.push_ask(Order {
            price: nz!(100),
            quantity: nz!(50),
            memo: 0,
        });

        let taker = Order {
            price: nz!(100),
            quantity: nz!(50),
            memo: 0,
        };
        let result =
            try_fill_orders(&mut orderbook, taker, OrderSide::Buy, OrderType::Limit).unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::Complete);
        assert_eq!(result.maker_fills.len(), 1);
        assert_eq!(result.maker_fills[0].fill_type, FillType::Complete);
    }

    #[test]
    fn test_partial_fill() {
        let mut orderbook = Orderbook::new();
        orderbook.push_ask(Order {
            price: nz!(100),
            quantity: nz!(30),
            memo: 0,
        });
        let taker = Order {
            price: nz!(100),
            quantity: nz!(50),
            memo: 0,
        };

        let result =
            try_fill_orders(&mut orderbook, taker, OrderSide::Buy, OrderType::Limit).unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::Partial);
        assert_eq!(result.maker_fills.len(), 1);
        assert_eq!(result.maker_fills[0].fill_type, FillType::Complete);
    }

    #[test]
    fn test_no_possible_fill() {
        let mut orderbook = Orderbook::new();
        orderbook.push_ask(Order {
            price: nz!(150),
            quantity: nz!(50),
            memo: 0,
        });
        let taker = Order {
            price: nz!(100),
            quantity: nz!(50),
            memo: 0,
        };

        let result =
            try_fill_orders(&mut orderbook, taker, OrderSide::Buy, OrderType::Limit).unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::None);
        assert_eq!(result.maker_fills.len(), 0);
    }

    #[test]
    fn test_no_matching_orders() {
        let mut orderbook = Orderbook::new();
        let taker = Order {
            price: nz!(100),
            quantity: nz!(50),
            memo: 0,
        };

        let result =
            try_fill_orders(&mut orderbook, taker, OrderSide::Buy, OrderType::Limit).unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::None);
        assert_eq!(result.maker_fills.len(), 0);
    }

    #[test]
    fn test_price_mismatch_for_limit_order() {
        let mut orderbook = Orderbook::new();
        orderbook.push_ask(Order {
            price: nz!(150),
            quantity: nz!(50),
            memo: 0,
        });
        let taker = Order {
            price: nz!(100),
            quantity: nz!(50),
            memo: 0,
        };

        let result =
            try_fill_orders(&mut orderbook, taker, OrderSide::Buy, OrderType::Limit).unwrap();
        assert_eq!(result.taker_fill_outcome, FillType::None);
        assert_eq!(result.maker_fills.len(), 0);
    }

    #[test]
    fn test_fulfillment_with_multiple_asks() {
        let mut orderbook = Orderbook::new();

        // Adding multiple sell orders at different prices and quantities
        orderbook.push_ask(Order {
            price: nz!(100),
            quantity: nz!(30),
            memo: 1,
        });
        orderbook.push_ask(Order {
            price: nz!(105),
            quantity: nz!(20),
            memo: 2,
        });
        orderbook.push_ask(Order {
            price: nz!(110),
            quantity: nz!(50),
            memo: 3,
        });

        let taker = Order {
            price: nz!(110),   // Taker is willing to buy up to this price
            quantity: nz!(75), // Taker wants a total of 75 units
            memo: 4,
        };

        let result =
            try_fill_orders(&mut orderbook, taker, OrderSide::Buy, OrderType::Limit).unwrap();

        // Assertions on overall outcome
        assert_eq!(result.taker_fill_outcome, FillType::Complete);
        assert_eq!(
            result.maker_fills.len(),
            3,
            "{maker_fills:#?}",
            maker_fills = result.maker_fills
        );

        // Assertions on individual fills - detailed assertion on each fill type
        assert_eq!(result.maker_fills[0].maker.price, nz!(100));
        assert_eq!(result.maker_fills[0].fill_type, FillType::Complete);
        assert_eq!(result.maker_fills[0].fill_amount, 30);

        assert_eq!(result.maker_fills[1].maker.price, nz!(105));
        assert_eq!(result.maker_fills[1].fill_type, FillType::Complete);
        assert_eq!(result.maker_fills[1].fill_amount, 20);

        assert_eq!(result.maker_fills[2].maker.price, nz!(110));
        assert_eq!(result.maker_fills[2].fill_type, FillType::Partial); // Correctly marked as Partial
        assert_eq!(result.maker_fills[2].fill_amount, 25);

        // Asserting the exact quantities and conditions met
        let total_filled_quantity = result
            .maker_fills
            .iter()
            .map(|fill| fill.fill_amount)
            .sum::<u32>();
        assert_eq!(
            total_filled_quantity, 75,
            "Total filled quantity should match the taker's required quantity."
        );

        // Assertions on the PendingFill structure
        assert_eq!(result.taker.price, nz!(110));
        assert_eq!(result.taker.quantity, nz!(75)); // Ensure original taker's quantity remains unchanged in the struct
        assert_eq!(result.side, OrderSide::Buy);
        assert_eq!(result.order_type, OrderType::Limit);
        assert_eq!(result.taker_fill_outcome, FillType::Complete);
    }
}
