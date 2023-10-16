use std::num::NonZeroU32;

use crate::trading::price_level::PriceLevel;

use super::order::{Order, OrderSide};
use super::price_level::MultiplePriceLevels;

/// An index into the [`Orderbook`] which can be used to identify an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderIndex {
    side: OrderSide,
    price: NonZeroU32,
    memo: u32,
}

pub struct Orderbook {
    pub(super) bids: MultiplePriceLevels,
    pub(super) asks: MultiplePriceLevels,
}

impl Orderbook {
    #[inline]
    #[track_caller]
    pub fn new() -> Self {
        let bids = MultiplePriceLevels {
            inner: tinyvec::tiny_vec!(),
        };
        let asks = MultiplePriceLevels {
            inner: tinyvec::tiny_vec!(),
        };
        Self { bids, asks }
    }

    #[inline]
    #[track_caller]
    pub fn push_bid(&mut self, t: Order) -> OrderIndex {
        let (price, memo) = self.bids.push_order_to_level(t);
        let side = OrderSide::Buy;
        OrderIndex { side, price, memo }
    }

    #[inline]
    #[track_caller]
    pub fn push_ask(&mut self, t: Order) -> OrderIndex {
        let (price, memo) = self.asks.push_order_to_level(t);
        let side = OrderSide::Sell;
        OrderIndex { side, price, memo }
    }

    #[inline]
    #[track_caller]
    pub fn remove(&mut self, order_index: OrderIndex) -> Option<Order> {
        let OrderIndex { side, price, memo } = order_index;

        let levels = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        levels.remove_order_from_level((price, memo))
    }

    /// construct an iterator of the side of the book specified, the ordering is relative depending on the side specified.
    ///
    /// * [`OrderSide::Buy`] - lowest price to highest
    /// * [`OrderSide::Sell`] - highest price to lowest
    ///
    pub fn iter_rel(&self, side: OrderSide) -> impl Iterator<Item = (OrderIndex, Order)> + '_ {
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

        fn wrap_iter<'a, I: Iterator<Item = &'a PriceLevel> + 'a>(
            side: OrderSide,
            iter: I,
        ) -> impl Iterator<Item = (OrderIndex, Order)> + 'a {
            iter.flat_map(move |level| {
                level.iter().copied().map(move |o| {
                    let order_index = OrderIndex {
                        side,
                        price: o.price,
                        memo: o.memo,
                    };
                    (order_index, o)
                })
            })
        }

        match side {
            OrderSide::Buy => Either::Left(wrap_iter(side, self.bids.iter_inner())),
            OrderSide::Sell => Either::Right(wrap_iter(side, self.asks.iter_inner_rev())),
        }
    }

    #[inline]
    #[track_caller]
    pub fn get_mut(&mut self, order_index: OrderIndex) -> Option<&mut Order> {
        let OrderIndex { side, price, memo } = order_index;

        let levels = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        levels.get_mut((price, memo))
    }
}
