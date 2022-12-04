use std::collections::VecDeque;
use std::num::NonZeroU16;

use ob::index::OrderIndex;
use ob::order::Order;

pub struct PlaceOrder {
    memo: u16,
    side: ob::order::OrderSide,
    quantity: NonZeroU16,
    price: NonZeroU16,
}

pub struct FetchOrder {
    ix: u64,
}

pub struct AmendOrder {
    ix: u64,
    memo: Option<u16>,
    quantity: Option<NonZeroU16>,
    price: Option<NonZeroU16>,
}

pub struct CancelOrder {
    ix: u64,
}

pub enum EngineInput {
    /// Create an order and attempt to fill resting.
    PlaceOrder(PlaceOrder),
    /// Get the details of an order i.e. quantity, price
    FetchOrder(FetchOrder),
    /// Modify the contents of an order on the book.
    AmendOrder(AmendOrder),
    /// Cancel an order, removing it from the book.
    CancelOrder(CancelOrder),
}

/// Bundle of state for the book and ask/bid queues.
pub struct EngineState {
    book: ob::book::Orderbook,
    ask_q: VecDeque<OrderIndex>,
    bid_q: VecDeque<OrderIndex>,
}

impl EngineState {
    /// creates a new book and bid/ask queues using `1MB` + `16KB` of pre-allocated space.
    pub fn new_with_capacity() -> Self {
        Self {
            book: ob::book::Orderbook::new(2), // 524KB x 2 = ~1MB
            ask_q: VecDeque::with_capacity(1024),
            bid_q: VecDeque::with_capacity(1024),
        }
    }

    /// wrap the engine state with some callback impl allowing placing,fetching,amending,cancelling orders.
    pub fn with_callbacks<'a, C>(&'a mut self, callbacks: C) -> EngineWithCallbacks<'a, C> {
        EngineWithCallbacks {
            engine: self,
            callbacks,
        }
    }
}

/// a set of callbacks to implement if you want to react to fill,update,cancel events.
pub trait Callbacks {
    /// a new aggressor order has entered the system.
    fn new_aggressor(&mut self, aggressor: OrderIndex);
    /// the aggressor order has been completely filled by the resting order.
    fn aggressor_fill(&mut self, aggressor: OrderIndex, resting: OrderIndex);
    /// the aggressor order has been partially filled by the resting order.
    fn aggressor_partial_fill(&mut self, aggressor: OrderIndex, resting: OrderIndex);
    /// the resting order has been completely filled by the aggressor order.
    fn resting_fill(&mut self, aggressor: OrderIndex, resting: OrderIndex);
    /// the resting order has been partially filled by the aggressor order.
    fn resting_partial_fill(&mut self, aggressor: OrderIndex, resting: OrderIndex);
    /// an aggressor order has turned into a resting order.
    fn aggressor_to_resting(&mut self, aggressor: OrderIndex);
    /// the resting order was updated and is equal to `body`.
    fn update_order(&mut self, resting: OrderIndex, body: Order);
    /// the order was marked as cancelled and has been removed from the book.
    fn cancel_order(&mut self, order_ix: OrderIndex, body: Order);
}

/// Used to modify the engine state.
pub struct EngineWithCallbacks<'ob, C> {
    engine: &'ob mut EngineState,
    callbacks: C,
}

/// An order was not present in the book.
pub struct OrderNotFound;

impl<'ob, C> EngineWithCallbacks<'ob, C>
where
    C: Callbacks,
{
    /// execute a [`CancelOrder`] removing the order in the book if it was present.
    #[inline]
    pub fn cancel_order(&mut self, cancel_order: CancelOrder) -> Result<OrderIndex, OrderNotFound> {
        let CancelOrder { ix } = cancel_order;

        let (oix, order) = self.engine.book.get_raw(ix).ok_or(OrderNotFound)?;
        let order = *order;

        self.engine.book.remove(oix);
        self.callbacks.cancel_order(oix, order);
        Ok(oix)
    }

    /// execute a [`AmendOrder`], modifying `memo`, `quantity`, and `price` fields if set.
    #[inline]
    pub fn amend_order(&mut self, amend_order: AmendOrder) -> Result<OrderIndex, OrderNotFound> {
        let AmendOrder {
            ix,
            memo,
            quantity,
            price,
        } = amend_order;

        let (oix, order_mut) = self.engine.book.get_raw_mut(ix).ok_or(OrderNotFound)?;

        if let Some(memo) = memo {
            *order_mut.memo_mut() = memo
        }

        if let Some(quantity) = quantity {
            *order_mut.quantity_mut() = quantity.get();
        }

        if let Some(price) = price {
            *order_mut.price_mut() = price.get();
        }

        self.callbacks.update_order(oix, *order_mut);

        Ok(oix)
    }

    /// execute a [`FetchOrder`] performing a constant time access of the book.
    ///
    /// Works at an `O(1)` complexity but will only respond with order details
    /// if the order is in the book and is still live.
    ///
    #[inline]
    pub fn fetch_order(&self, fetch_order: FetchOrder) -> Option<(OrderIndex, &Order)> {
        let FetchOrder { ix } = fetch_order;
        self.engine.book.get_raw(ix)
    }

    /// execute a [`PlaceOrder`] potentially filling multiple resting orders and the aggressor.
    ///
    /// the fill traverses the internal bid/ask queue linearly from front to
    /// back.
    ///
    /// askers will buy at the price or better, bidders will sell at the price or better.
    ///
    /// NOTE: no implicit time/price priority is performed, the queues must be sorted
    /// accordingly if you want them to.
    ///
    #[inline]
    pub fn place_order(&mut self, place_order: PlaceOrder) -> Option<OrderIndex> {
        let PlaceOrder {
            memo,
            side,
            quantity: qty,
            price,
        } = place_order;

        let (a_order, is_ask) = match side {
            ob::order::OrderSide::Ask => {
                (ob::order::Order::ask(memo, qty.get(), price.get()), true)
            }

            ob::order::OrderSide::Bid => {
                (ob::order::Order::bid(memo, qty.get(), price.get()), false)
            }
        };

        debug_assert_eq!(a_order.quantity(), qty.get());
        debug_assert_eq!(a_order.price(), price.get());

        let a_order_index = self.engine.book.insert(a_order);

        let mut a_qty = qty.get();
        let book = &mut self.engine.book;

        let (queue, price_range) = if is_ask {
            // bid_q = list of sellers, we are buying at same price or lower
            (self.engine.bid_q.iter(), 0..(price.get() + 1))
        } else {
            // ask_q = list of buyers, we are selling at same price or higher
            (self.engine.ask_q.iter(), price.get()..u16::MAX)
        };

        for r_oix in queue {
            let r_order = match book.get_mut(*r_oix) {
                Some(o) => o,
                None => continue,
            };

            let r_qty = r_order.quantity();

            if !price_range.contains(&r_order.price()) {
                continue;
            }

            // [aggressor] complete fill, resting order is larger.
            if a_qty < r_qty {
                *r_order.quantity_mut() = r_qty - a_qty;
                book.remove(a_order_index);
                self.callbacks.aggressor_fill(a_order_index, *r_oix);
                self.callbacks.resting_partial_fill(a_order_index, *r_oix);
                break;
            }
            // [aggressor, resting] complete fill, orders are the same size.
            else if a_qty == r_qty {
                book.remove(*r_oix);
                book.remove(a_order_index);
                self.callbacks.aggressor_fill(a_order_index, *r_oix);
                self.callbacks.resting_fill(a_order_index, *r_oix);
                break;
            }
            // [aggressor] partial fill, aggressor order is larger.
            else {
                assert!(a_qty > r_qty);
                a_qty -= r_qty;
                book.remove(*r_oix);
                self.callbacks.resting_fill(a_order_index, *r_oix);
                self.callbacks.aggressor_partial_fill(a_order_index, *r_oix);
            }
        }

        if a_qty > 0 {
            // not complete fill, become resting order.
            self.callbacks.aggressor_to_resting(a_order_index);
            Some(a_order_index)
        } else {
            // complete fill.
            None
        }
    }
}
