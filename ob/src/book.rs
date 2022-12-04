use crate::{index::OrderIndex, order::Order};

union InnerEntry {
    order: Order,
    index: OrderIndex,
}

impl InnerEntry {
    #[inline]
    fn order_mut(&mut self) -> Option<&mut Order> {
        if unsafe { self.order.is_invalid() } {
            None
        } else {
            Some(unsafe { &mut self.order })
        }
    }

    #[inline]
    fn order(&self) -> Option<&Order> {
        // both types are u64s and the first two bytes are non-null for orders.
        if unsafe { self.order.is_invalid() } {
            None
        } else {
            // safety: valid orders are invalid indecies
            Some(unsafe { &self.order })
        }
    }

    #[inline]
    fn index(&self) -> Option<OrderIndex> {
        self.order().and_then(|_| None::<OrderIndex>).or_else(|| {
            // safety: invalid orders are valid indecies
            Some(unsafe { self.index })
        })
    }
}

/// implementation of the orderbook storage.
struct Inner {
    last_removed: u32,
    inner_tail_offset: usize,
    inner: Vec<InnerEntry>,
}

impl Inner {
    fn new(array: Vec<InnerEntry>) -> Self {
        Self {
            inner: array,
            inner_tail_offset: 0,
            last_removed: u32::MAX,
        }
    }

    #[inline]
    pub fn push(&mut self, t: Order) -> OrderIndex {
        self.inner.push(InnerEntry { order: t });
        let ix = self.inner.len().saturating_sub(1);
        OrderIndex::new(ix as u32, 0)
    }

    #[inline]
    pub fn remove(&mut self, oix: OrderIndex) {
        let index = oix.index() as usize;

        if let Some(entry) = self.inner.get_mut(index) {
            // SAFETY: `OrderIndex` and `Order` have the same size and alignment requirements
            *entry = unsafe { std::mem::transmute(oix) };
            self.last_removed = index as u32;
        }
    }
}

pub struct Orderbook {
    inner: Inner,
}

impl Orderbook {
    /// create a new orderbook with a pre-allocated amount of "pages"
    ///
    /// every page is made up of [`u16::MAX`] rows, which means each
    /// page contains `65,535` row and an individual row is 8 bytes large.
    ///
    /// one page works out to `524,280` bytes or `524.2` kilobytes.
    ///
    pub fn new(pages: u16) -> Self {
        let capacity = (pages * u16::MAX) as usize;

        Self {
            inner: Inner::new(Vec::with_capacity(capacity)),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Order> + '_ {
        self.inner
            .inner
            .iter()
            .skip(self.inner.inner_tail_offset)
            .filter_map(|entry| entry.order())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Order> {
        self.inner
            .inner
            .iter_mut()
            .skip(self.inner.inner_tail_offset)
            .filter_map(|entry| entry.order_mut())
    }

    pub fn get_raw(&self, raw: u64) -> Option<(OrderIndex, &Order)> {
        let oix = OrderIndex(raw);
        let ix = oix.index() as usize;

        if ix < self.inner.inner_tail_offset {
            return None;
        }

        let other = self.inner.inner.get(ix)?.order();
        Some(oix).zip(other)
    }

    pub fn get_raw_mut(&mut self, raw: u64) -> Option<(OrderIndex, &mut Order)> {
        let oix = OrderIndex(raw);
        let ix = oix.index() as usize;

        if ix < self.inner.inner_tail_offset {
            return None;
        }

        let other = self.inner.inner.get_mut(ix)?.order_mut();
        Some(oix).zip(other)
    }

    /// retrieve an order by its associated index.
    #[inline]
    pub fn get(&self, order_ix: OrderIndex) -> Option<&Order> {
        let ix = order_ix.index() as usize;

        if ix < self.inner.inner_tail_offset {
            return None;
        }

        self.inner.inner.get(ix)?.order()
    }

    /// retrieve an order by its associated index.
    #[inline]
    pub fn get_mut(&mut self, order_ix: OrderIndex) -> Option<&mut Order> {
        let ix = order_ix.index() as usize;

        if ix < self.inner.inner_tail_offset {
            return None;
        }

        self.inner.inner.get_mut(ix)?.order_mut()
    }

    /// remove an order with some associated index.
    #[inline]
    pub fn remove(&mut self, order_ix: OrderIndex) {
        if self.inner.inner_tail_offset == order_ix.index() as usize {
            self.inner.inner_tail_offset += 1;
        }

        self.inner.remove(order_ix) // NOTE: last_removed is set internally
    }

    /// insert an order and generate an index for it.
    ///
    /// This will attempt to reuse the memory of recently deleted orders removed with [`Orderbook::remove`]
    /// rather than inserting new rows every time. The benefit of this is that locallity of reference is
    /// preserved and we will most likely be writing and using memory already in cache.
    ///
    #[inline]
    pub fn insert(&mut self, order: Order) -> OrderIndex {
        let tail_offset = self.inner.inner_tail_offset;

        // check if there is a last_removed index set, use that row and update the index to the tail offset if possible.
        if self.inner.last_removed != u32::MAX {
            let slim_order = &mut self.inner.inner[self.inner.last_removed as usize];
            let entry = std::mem::replace(slim_order, InnerEntry { order });

            // SAFETY: `last_removed` is always an index of a transmuted `OrderIndex`
            let ix: OrderIndex = entry.index().expect("last_removed entry was not an index");
            let index = ix.index() as usize;

            self.inner.last_removed = if tail_offset > 0 {
                let diff = if tail_offset == index.saturating_add(1) {
                    self.inner.inner_tail_offset = index;
                    2
                } else {
                    1
                };

                tail_offset.saturating_sub(diff) as u32
            } else {
                u32::MAX
            };

            ix.inc_gen()
        } else {
            // no last_removed slot, and tail index is zero otherwise last_removed would be set.
            self.inner.push(order)
        }
    }
}
