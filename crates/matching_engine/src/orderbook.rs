//! The orderbook module contains the data structures and logic for the orderbook.
use crate::decimal::NonZeroDecimal;
use crate::order_ticket::OrderTicket;
use crate::order_uuid::OrderUuid;

/// Side of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderSide {
    /// Buy/"Bid"
    #[serde(rename = "buy")]
    Buy,
    /// Sell/"Ask"
    #[serde(rename = "sell")]
    Sell,
}

/// The execution model of the order.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderType {
    /// The full order quantity is placed immediately with a limit price restriction to only trade at this price or better.
    #[cfg_attr(feature = "serde", serde(rename = "limit"))]
    Limit,
    /// The full order quantity executes immediately at the best available price in the order book.
    #[cfg_attr(feature = "serde", serde(rename = "market"))]
    Market,
    /// A market order is triggered when the reference price reaches the stop price (from an unfavourable direction).
    #[cfg_attr(feature = "serde", serde(rename = "stop-loss"))]
    StopLoss,
    /// A limit order is triggered when the reference price reaches the stop price (from an unfavourable direction).
    #[cfg_attr(feature = "serde", serde(rename = "stop-loss-limit"))]
    StopLossLimit,
    /// A market order is triggered when the reference price reaches the stop price (from an favourable direction).
    #[cfg_attr(feature = "serde", serde(rename = "take-profit"))]
    TakeProfit,
    /// A limit order is triggered when the reference price reaches the stop price (from an favourable direction).
    #[cfg_attr(feature = "serde", serde(rename = "take-profit-limit"))]
    TakeProfitLimit,
    /// A market order is triggered when the market reverts a specified distance from the peak price.
    #[cfg_attr(feature = "serde", serde(rename = "trailing-stop"))]
    TrailingStop,
    /// A limit order is triggered when the market reverts a specified distance from the peak price.
    #[cfg_attr(feature = "serde", serde(rename = "trailing-stop-limit"))]
    TrailingStopLimit,
    /// Hides the full order size by only showing your chosen display size in the book at your limit price.
    #[cfg_attr(feature = "serde", serde(rename = "iceberg"))]
    Iceberg,
}

/// The self-trade protection of an order.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SelfTradeProtection {
    /// resting order will be canceled
    #[serde(rename = "co")]
    CancelOldest,
    /// arriving order will be canceled
    #[serde(rename = "cn")]
    CancelNewest,
    /// both arriving and resting orders will be canceled
    #[serde(rename = "cb")]
    CancelBoth,
}

impl Default for SelfTradeProtection {
    fn default() -> Self {
        Self::CancelNewest // Default to safest option: skip self-trades
    }
}

type Date = u64;

/// Time in force options for orders.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimeInForce {
    /// Good Til Canceled, default. The order will remain open until it is either filled or canceled.
    #[cfg_attr(feature = "serde", serde(rename = "GTC", alias = "gtc"))]
    GoodTilCanceled,
    /// Good Til Date specified. The order will remain open until it is either filled or canceled. it will automatically cancel at the specified timestamp.
    #[cfg_attr(feature = "serde", serde(rename = "GTD", alias = "gtd"))]
    GoodTilDate(#[cfg_attr(feature = "serde", serde(skip))] Date),
    /// Immediate Or Cancel. The order must be filled immediately and any unfilled portion of the order will be canceled.
    #[cfg_attr(feature = "serde", serde(rename = "IOC", alias = "ioc"))]
    ImmediateOrCancel,
    /// Fill Or Kill. The order must be filled immediately in its entirety or it will be canceled. The difference between this and IOC is that GTC orders will be placed on the book until canceled instead of being canceled at the end of the trading day.
    #[cfg_attr(feature = "serde", serde(rename = "FOK", alias = "fok"))]
    FillOrKill,
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GoodTilCanceled
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct OrderData<TUserId> {
    /// A distinct number for this bit of data.
    pub timestamp: u32,
    /// Price at which the order is placed.
    pub price: NonZeroDecimal,
    /// unfilled part of the position
    pub remaining_quantity: NonZeroDecimal,
    /// filled part of the position
    pub filled_quantity: crate::decimal::Decimal,
    /// The unique identifier for the order.
    pub order_id: OrderUuid,
    /// For Iceberg orders: the visible quantity in the book (None for regular orders)
    pub display_quantity: Option<NonZeroDecimal>,
    /// User ID of the order owner (for self-trade protection)
    pub user_id: TUserId,
    /// userref is
    pub userref: Option<u32>,
}

/// Price-level aggregation storing multiple price levels in one contiguous array.
struct MultiplePriceLevels<TUserId> {
    counter: u32,
    /// All resting orders in the book in one contiguous array.
    orders: Vec<OrderData<TUserId>>,
}

impl<TUserId> MultiplePriceLevels<TUserId> {
    /// insert the order into its price level range in the book
    #[track_caller]
    fn insert_order(
        &mut self,
        price: NonZeroDecimal,
        order_id: OrderUuid,
        user_id: TUserId,
        details: OrderTicket,
    ) -> (usize, u32) {
        let remaining_quantity = details
            .quantity
            .expect("invariant: quantity must be non-zero and checked before inserting");
        let display_quantity = details.display_quantity;

        // invariant: display_quantity must not exceed remaining_quantity
        if let Some(display_qty) = display_quantity {
            assert!(
                display_qty <= remaining_quantity,
                "invariant: display_quantity ({}) cannot exceed remaining_quantity ({})",
                *display_qty,
                *remaining_quantity
            );
        }

        // invariant: counter must not overflow to maintain timestamp uniqueness
        assert!(
            self.counter < u32::MAX,
            "invariant: timestamp counter overflow would break uniqueness"
        );
        let timestamp = self.counter + 1;
        self.counter += 1;

        match self.orders.binary_search_by(
            |probe: &OrderData<TUserId>| {
                probe
                    .price
                    .cmp(&price)
                    .then(probe.timestamp.cmp(&timestamp))
            }, // this should "swing right" so we can insert at the end of the price level
        ) {
            Ok(index) => unreachable!(
                "not possible for an order in the price level to have the same timestamp as a new order"
            ),
            Err(index) => {
                self.orders.insert(
                    index,
                    OrderData {
                        timestamp: timestamp,
                        price,
                        remaining_quantity,
                        filled_quantity: crate::decimal::Decimal::ZERO,
                        order_id,
                        display_quantity,
                        user_id,
                        userref: details.userref,
                    },
                );

                // invariant: verify global price-time ordering is maintained
                for (ix, order) in self.orders.iter().enumerate() {
                    if ix > 0 {
                        let prev = &self.orders[ix - 1];
                        assert!(
                            prev.price < order.price
                                || (prev.price == order.price && prev.timestamp < order.timestamp),
                            "invariant: orders must be sorted by price then timestamp at index {} (prev: {:?}@{}, current: {:?}@{})",
                            ix,
                            prev.price,
                            prev.timestamp,
                            order.price,
                            order.timestamp
                        );
                    }
                }

                (index, timestamp)
            }
        }
    }

    fn remove_order(
        &mut self,
        price: NonZeroDecimal,
        timestamp: u32,
    ) -> Option<OrderData<TUserId>> {
        if let Ok(index) = self.orders.binary_search_by(|probe| {
            probe
                .price
                .cmp(&price)
                .then(probe.timestamp.cmp(&timestamp))
        }) {
            Some(self.orders.remove(index))
        } else {
            None
        }
    }

    fn get(&self, price: NonZeroDecimal, timestamp: u32) -> Option<&OrderData<TUserId>> {
        if let Ok(index) = self.orders.binary_search_by(|probe| {
            probe
                .price
                .cmp(&price)
                .then(probe.timestamp.cmp(&timestamp))
        }) {
            self.orders.get(index)
        } else {
            None
        }
    }

    fn get_mut(
        &mut self,
        price: NonZeroDecimal,
        timestamp: u32,
    ) -> Option<&mut OrderData<TUserId>> {
        if let Ok(index) = self.orders.binary_search_by(|probe| {
            probe
                .price
                .cmp(&price)
                .then(probe.timestamp.cmp(&timestamp))
        }) {
            self.orders.get_mut(index)
        } else {
            None
        }
    }
}

/// An index into the [`Orderbook`] which can be used to identify an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderIndex {
    pub side: OrderSide,
    pub price: NonZeroDecimal,
    pub timestamp: u32,
}

/// Central Limit Order Book storing orders in price-time priority (price levels with FIFO ordering within each level)
pub struct Orderbook<TUserId> {
    /// bids side of the book
    bids: MultiplePriceLevels<TUserId>,
    /// asks side of the book
    asks: MultiplePriceLevels<TUserId>,
}

impl<TUserId> Orderbook<TUserId> {
    pub fn new_empty() -> Self {
        let bids = MultiplePriceLevels {
            orders: vec![],
            counter: 0,
        };
        let asks = MultiplePriceLevels {
            orders: vec![],
            counter: 0,
        };
        Self { bids, asks }
    }

    pub fn insert(
        &mut self,
        price: NonZeroDecimal,
        order_id: OrderUuid,
        user_id: TUserId,
        details: OrderTicket,
    ) -> OrderIndex {
        let side = details.side;
        let levels = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        let (_index, timestamp) = levels.insert_order(price, order_id, user_id, details);
        OrderIndex {
            side,
            price,
            timestamp,
        }
    }

    /// Insert an amended order with preserved filled_quantity
    pub fn insert_amended(
        &mut self,
        side: OrderSide,
        price: NonZeroDecimal,
        order_id: OrderUuid,
        user_id: TUserId,
        remaining_quantity: NonZeroDecimal,
        filled_quantity: crate::decimal::Decimal,
        display_quantity: Option<NonZeroDecimal>,
        userref: Option<u32>,
    ) -> OrderIndex {
        let levels = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        // Display quantity validation
        if let Some(display_qty) = display_quantity {
            assert!(
                display_qty <= remaining_quantity,
                "display_quantity cannot exceed remaining_quantity"
            );
        }

        // Get new timestamp
        assert!(levels.counter < u32::MAX, "timestamp counter overflow");
        let timestamp = levels.counter + 1;
        levels.counter += 1;

        // Find insertion point
        let index = levels
            .orders
            .binary_search_by(|probe: &OrderData<TUserId>| {
                probe
                    .price
                    .cmp(&price)
                    .then(probe.timestamp.cmp(&timestamp))
            })
            .unwrap_err(); // Should always be Err since timestamp is unique

        // Insert the order
        levels.orders.insert(
            index,
            OrderData {
                timestamp,
                price,
                remaining_quantity,
                filled_quantity,
                order_id,
                display_quantity,
                user_id,
                userref,
            },
        );

        OrderIndex {
            side,
            price,
            timestamp,
        }
    }

    pub fn remove(&mut self, order_index: OrderIndex) -> Option<OrderData<TUserId>> {
        let OrderIndex {
            side,
            price,
            timestamp,
        } = order_index;

        let levels = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        levels.remove_order(price, timestamp)
    }

    pub fn get(&self, order_index: OrderIndex) -> Option<&OrderData<TUserId>> {
        let OrderIndex {
            side,
            price,
            timestamp,
        } = order_index;

        let levels = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };

        levels.get(price, timestamp)
    }

    pub fn get_mut(&mut self, order_index: OrderIndex) -> Option<&mut OrderData<TUserId>> {
        let OrderIndex {
            side,
            price,
            timestamp,
        } = order_index;

        let levels = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        levels.get_mut(price, timestamp)
    }

    pub fn bids(
        &self,
    ) -> std::iter::Rev<std::iter::Enumerate<std::slice::Iter<'_, OrderData<TUserId>>>> {
        self.bids.orders.iter().enumerate().rev()
    }

    pub fn asks(&self) -> std::iter::Enumerate<std::slice::Iter<'_, OrderData<TUserId>>> {
        self.asks.orders.iter().enumerate()
    }

    /// Get the best (lowest) ask price, if any orders exist on the ask side
    pub fn best_ask_price(&self) -> Option<NonZeroDecimal> {
        self.asks.orders.first().map(|order| order.price)
    }

    /// Get the best (highest) bid price, if any orders exist on the bid side
    pub fn best_bid_price(&self) -> Option<NonZeroDecimal> {
        self.bids.orders.last().map(|order| order.price)
    }

    /// construct an iterator of the side of the book specified, the ordering is relative depending on the side specified.
    ///
    /// * [`OrderSide::Buy`] - highest price to lowest (for selling)
    /// * [`OrderSide::Sell`] - lowest price to highest (for buying)
    ///
    pub(crate) fn iter_for_limit_relative(
        &self,
        side: OrderSide,
        limit_price: NonZeroDecimal,
    ) -> impl Iterator<Item = (usize, &OrderData<TUserId>)> + '_ {
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

        match side {
            OrderSide::Buy => Either::Left({
                let limit_start = self
                    .bids
                    .orders
                    .binary_search_by(|probe| probe.price.cmp(&limit_price))
                    .unwrap_or_else(|ix| ix);

                self.bids.orders[limit_start..].iter().enumerate().rev()
            }),
            OrderSide::Sell => Either::Right({
                let limit_end = self
                    .asks
                    .orders
                    .binary_search_by(|probe| probe.price.cmp(&limit_price))
                    .map(|ix| ix + 1) // include the price level itself
                    .unwrap_or_else(|ix| ix); // exclude the insertion point

                self.asks.orders[..limit_end].iter().enumerate()
            }),
        }
    }
}

#[cfg(all(feature = "serde", test))]
mod test_serde {
    use super::*;

    #[test]
    fn test_de_order_type() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct Shim {
            t: OrderType,
        }

        macro_rules! d {
            ($string:literal) => {
                ::serde_json::from_str::<Shim>($string).unwrap()
            };
        }
        assert_eq!(
            d!("{\"t\": \"limit\"}"),
            Shim {
                t: OrderType::Limit
            }
        );
        assert_eq!(
            d!("{\"t\": \"take-profit\"}"),
            Shim {
                t: OrderType::TakeProfit
            }
        );
        assert_eq!(
            d!("{\"t\": \"take-profit-limit\"}"),
            Shim {
                t: OrderType::TakeProfitLimit
            }
        );
        assert_eq!(
            d!("{\"t\": \"trailing-stop\"}"),
            Shim {
                t: OrderType::TrailingStop
            }
        );
        assert_eq!(
            d!("{\"t\": \"trailing-stop-limit\"}"),
            Shim {
                t: OrderType::TrailingStopLimit
            }
        );
        assert_eq!(
            d!("{\"t\": \"iceberg\"}"),
            Shim {
                t: OrderType::Iceberg
            }
        );
    }
}
