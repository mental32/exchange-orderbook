use std::num::NonZeroU32;

use tinyvec::{tiny_vec, TinyVec};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum OrderSide {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "limit")]
    Limit,
    #[serde(rename = "market")]
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Order {
    pub(super) memo: u32,
    pub(super) quantity: NonZeroU32,
    pub(super) price: NonZeroU32,
}

impl Order {
    #[inline]
    pub fn ask(quantity: NonZeroU32, price: NonZeroU32) -> Self {
        Self {
            memo: 0,
            quantity,
            price,
        }
    }

    #[inline]
    pub fn bid(quantity: NonZeroU32, price: NonZeroU32) -> Self {
        Self {
            memo: 0,
            quantity,
            price,
        }
    }

    #[inline]
    pub fn quantity(&self) -> NonZeroU32 {
        self.quantity
    }

    #[inline]
    pub fn price(&self) -> NonZeroU32 {
        self.price
    }
}

/// The threshold at which the [`PriceLevel`] will switch from using array storage to heap storage.
const PRICE_LEVEL_INNER_CAPACITY: usize = 64;

/// The inner data structure for a [`MultiplePriceLevels`].
#[derive(Debug, Default)]
pub struct PriceLevel {
    /// The price of the orders in this price level.
    price: u32,
    /// The sequence number generator for the next order to be added to this price level.
    memo_seq: u32,
    /// The inner data structure storing the orders in this price level.
    inner: TinyVec<[Option<Order>; PRICE_LEVEL_INNER_CAPACITY]>,
}

impl PriceLevel {
    #[track_caller]
    pub fn iter(&self) -> impl Iterator<Item = &Order> + '_ {
        self.inner
            .iter()
            .map(|o| o.as_ref().expect("all valid orders are always Some"))
    }

    #[inline]
    #[track_caller]
    fn push_order(&mut self, mut t: Order) -> (NonZeroU32, u32) {
        let price = NonZeroU32::new(self.price).expect("price for price-level should not be zero");
        let memo = self.memo_seq;
        self.memo_seq += 1;
        t.memo = memo;
        let rval = (price, memo);
        self.inner.push(Some(t));
        rval
    }

    fn remove_order(&mut self, memo: u32) -> Option<Order> {
        let index = self.iter().position(|o| o.memo == memo)?;
        let rval = self.inner.remove(index);

        if self.inner.len() <= PRICE_LEVEL_INNER_CAPACITY {
            self.inner.shrink_to_fit();
        }

        rval
    }
}

/// The threshold at which the [`MultiplePriceLevels`] will switch from using array storage to heap storage.
pub const MULTIPLE_PRICE_LEVEL_INNER_CAPACITY: usize = 128;

/// Stores multiple price levels in a contiguous vector.
pub struct MultiplePriceLevels {
    pub(super) inner: TinyVec<[PriceLevel; MULTIPLE_PRICE_LEVEL_INNER_CAPACITY]>,
}

impl MultiplePriceLevels {
    /// Returns an iterator over the [`PriceLevel`]s in the [`MultiplePriceLevels`] in decenting order.
    pub(crate) fn iter_inner_rev(&self) -> impl Iterator<Item = &PriceLevel> + '_ {
        self.inner.iter().rev()
    }

    /// Returns an iterator over the [`PriceLevel`]s in the [`MultiplePriceLevels`] in ascending order.
    pub(crate) fn iter_inner(&self) -> impl Iterator<Item = &PriceLevel> + '_ {
        self.inner.iter()
    }

    /// Returns the [`PriceLevel`] for the given price.
    pub fn get_or_insert_price_level(&mut self, price: NonZeroU32) -> &mut PriceLevel {
        let index = self
            .inner
            .binary_search_by_key(&price.get(), |level| level.price);

        match index {
            Ok(index) => self.inner.get_mut(index).expect("checked index"),
            Err(index) => {
                self.inner.insert(
                    index,
                    PriceLevel {
                        price: price.get(),
                        memo_seq: 0,
                        inner: tiny_vec!(),
                    },
                );
                self.inner.get_mut(index).expect("checked index")
            }
        }
    }

    /// Pushes an order to the [`MultiplePriceLevels`] returns a tuple of the price and memo of the order.
    pub fn push_order_to_level(&mut self, t: Order) -> (NonZeroU32, u32) {
        let index = self
            .inner
            .binary_search_by_key(&t.price.get(), |level| level.price);

        match index {
            Ok(index) => {
                let price_level = self.inner.get_mut(index);
                price_level.expect("checked index").push_order(t)
            }
            Err(index) => {
                let mut price_level_inner = PriceLevel {
                    price: t.price.get(),
                    memo_seq: 0,
                    inner: tiny_vec!(),
                };
                let ret = price_level_inner.push_order(t);
                self.inner.insert(index, price_level_inner);
                ret
            }
        }
    }

    /// Removes an order from the [`MultiplePriceLevels`] returns the order if it existed.
    pub fn remove_order_from_level(&mut self, (price, memo): (NonZeroU32, u32)) -> Option<Order> {
        let price_level_index = self
            .inner
            .binary_search_by_key(&price.get(), |level| level.price)
            .ok()?;

        let price_level = self.inner.get_mut(price_level_index)?;
        let order = price_level.remove_order(memo);

        if price_level.inner.is_empty() {
            self.inner.remove(price_level_index);
            if self.inner.len() <= MULTIPLE_PRICE_LEVEL_INNER_CAPACITY {
                self.inner.shrink_to_fit();
            }
        }

        order
    }

    /// Returns a mutable reference to an [`Order`] in the [`MultiplePriceLevels`] if it exists.
    pub fn get_mut(&mut self, (price, memo): (NonZeroU32, u32)) -> Option<&mut Order> {
        let index = self
            .inner
            .binary_search_by_key(&price.get(), |level| level.price)
            .ok()?;

        self.inner
            .get_mut(index)?
            .inner
            .iter_mut()
            .find(|o| matches!(o, Some(o) if o.memo == memo))
            .map(|o| o.as_mut().expect("checked order"))
    }
}

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
