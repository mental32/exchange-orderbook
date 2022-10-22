#![feature(allocator_api)]
#![feature(btreemap_alloc)]
#![feature(int_roundings)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OrderSide {
    Ask,
    Bid,
}

/// A memory-efficient order type.
///
/// Internally it is backed by a u8 array: `[u8; 8]`
///
/// ```
/// struct Order {
///  metadata: u16,
///  memo: u16,
///  quantity: u16,
///  price: u16
/// }
/// ```
///
/// The first octet is the metadata and contains the following information:
/// * bit 0: alive (1) or dead (0)
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Order([u8; 8]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct OrderMetadata(u8);

pub const SIDE_BIT: u8 = 0b0000_0010;
pub const DEAD_BIT: u8 = 0b0000_0001;

impl OrderMetadata {
    #[inline]
    pub fn side(&self) -> OrderSide {
        let side = self.0 & SIDE_BIT;

        match side {
            0 => OrderSide::Ask,
            1 => OrderSide::Bid,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn is_dead(&self) -> bool {
        match self.0 & DEAD_BIT {
            0 => false,
            1 => true,
            _ => unreachable!(),
        }
    }
}

impl Order {
    #[inline]
    pub fn metadata(&self) -> OrderMetadata {
        OrderMetadata(self.0[0])
    }

    #[inline]
    pub fn set_dead(&mut self, dead: bool) {
        let m = &mut self.0[0];
        if dead {
            *m |= DEAD_BIT;
        } else {
            *m &= !DEAD_BIT;
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct OrderIndex(u64);

impl OrderIndex {
    #[inline]
    const fn new(linear_index: u32, gen: u16) -> Self {
        let mut data = [
            0, // ZERO
            0, // ZERO
            0, // liner index
            0, // |
            0, // |
            0, // |
            0, // generation counter
            0, // |
        ];

        {
            let [a, b, c, d] = u32::to_le_bytes(linear_index);
            data[2] = a;
            data[3] = b;
            data[4] = c;
            data[5] = d;
        }

        {
            let [a, b] = u16::to_le_bytes(gen);
            data[6] = a;
            data[7] = b;
        }

        Self(u64::from_le_bytes(data))
    }

    #[inline]
    const fn to_bytes(&self) -> [u8; 8] {
        u64::to_le_bytes(self.0)
    }

    #[inline]
    const fn index(&self) -> u32 {
        let [_, _, a, b, c, d, _, _] = self.to_bytes();
        u32::from_le_bytes([a, b, c, d])
    }

    #[inline]
    const fn inc_gen(&self) -> OrderIndex {
        let [.., g1, g2] = self.to_bytes();
        let gen = u16::from_le_bytes([g1, g2]).saturating_add(1);
        Self::new(self.index(), gen)
    }
}

/// an array-like container that indexes stored types.
struct Inner {
    last_removed: u32,
    inner_tail_offset: usize,
    inner: Vec<Order>,
}

impl Inner {
    fn new(array: Vec<Order>) -> Self {
        Self {
            inner: array,
            inner_tail_offset: 0,
            last_removed: u32::MAX,
        }
    }

    #[inline]
    pub fn push(&mut self, t: Order) -> OrderIndex {
        self.inner.push(t);
        let ix = self.inner.len().saturating_sub(1);
        OrderIndex::new(ix as u32, 0)
    }

    #[inline]
    pub fn remove(&mut self, oix: OrderIndex) {
        let index = oix.index() as usize;

        if let Some(entry) = self.inner.get_mut(index) {
            // SAFETY: `OrderIndex` and `SlimOrder` have the same size and alignment requirements
            //
            *entry = unsafe { std::mem::transmute(oix) };
            self.last_removed = index as u32;
        }
    }
}

pub struct Orderbook {
    orders: Inner,
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
            orders: Inner::new(Vec::with_capacity(capacity)),
        }
    }

    /// retrieve an order by its associated index.
    #[inline]
    pub fn get(&self, order_ix: OrderIndex) -> Option<&Order> {
        let ix = order_ix.index() as usize;

        if ix < self.orders.inner_tail_offset {
            return None;
        }

        let o @ Order([m1, m2, ..]) = self.orders.inner.get(ix)?;

        if *m1 == 0 && *m2 == 0 {
            None
        } else {
            Some(o)
        }
    }

    /// remove an order with some associated index.
    #[inline]
    pub fn remove(&mut self, order_ix: OrderIndex) {
        if self.orders.inner_tail_offset == order_ix.index() as usize {
            self.orders.inner_tail_offset += 1;
        }

        self.orders.remove(order_ix) // NOTE: last_removed is set internally
    }

    /// insert an order and generate an index for it.
    ///
    /// This will attempt to reuse the memory of recently deleted orders removed with [`Orderbook::remove`]
    /// rather than inserting new rows every time. The benefit of this is that locallity of reference is
    /// preserved and we will most likely be writing and using memory already in cache.
    ///
    #[inline]
    pub fn insert(&mut self, order: Order) -> OrderIndex {
        let tail_offset = self.orders.inner_tail_offset;

        // check if there is a last_removed index set, use that row and update the index to the tail offset if possible.
        if self.orders.last_removed != u32::MAX {
            let slim_order = &mut self.orders.inner[self.orders.last_removed as usize];
            let entry = std::mem::replace(slim_order, order);

            // SAFETY: `last_removed` is always an index of a transmuted `OrderIndex`
            let ix: OrderIndex = unsafe { std::mem::transmute(entry) };
            let index = ix.index() as usize;

            self.orders.last_removed = if tail_offset > 0 {
                let diff = if tail_offset == index.saturating_add(1) {
                    self.orders.inner_tail_offset = index;
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
            self.orders.push(order)
        }
    }
}
