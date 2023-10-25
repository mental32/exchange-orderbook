use std::convert::Infallible;

use super::*;

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

    let maker_side = match side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    };

    match order_type {
        OrderType::Limit => {
            for (oix, order) in orderbook.iter_rel(maker_side) {
                match side {
                    OrderSide::Buy => {
                        if order.price > taker.price {
                            taker_fill_outcome = FillType::Partial;
                            break;
                        }
                    }
                    OrderSide::Sell => {
                        if order.price < taker.price {
                            taker_fill_outcome = FillType::Partial;
                            break;
                        }
                    }
                }

                if order.quantity == taker.quantity {
                    // The taker order is completely filled.
                    maker_fills.push((oix, order.clone(), FillType::Complete));
                    taker_fill_outcome = FillType::Complete;
                    break;
                } else if order.quantity < taker.quantity {
                    // The taker order is partially filled.
                    maker_fills.push((oix, order.clone(), FillType::Complete));
                } else {
                    assert!(order.quantity > taker.quantity);
                    // The taker order is completely filled.
                    maker_fills.push((oix, order.clone(), FillType::Partial));
                    taker_fill_outcome = FillType::Complete;
                    break;
                }
            }
        }
        OrderType::Market => {
            for (oix, order) in orderbook.iter_rel(maker_side) {
                if order.quantity == taker.quantity {
                    // The taker order is completely filled.
                    maker_fills.push((oix, order.clone(), FillType::Complete));
                    taker_fill_outcome = FillType::Complete;
                    break;
                } else if order.quantity < taker.quantity {
                    // The taker order is partially filled.
                    maker_fills.push((oix, order.clone(), FillType::Complete));
                } else {
                    assert!(order.quantity > taker.quantity);
                    // The taker order is completely filled.
                    maker_fills.push((oix, order.clone(), FillType::Partial));
                    taker_fill_outcome = FillType::Complete;
                    break;
                }
            }
        }
    };

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
