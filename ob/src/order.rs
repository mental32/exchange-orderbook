#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OrderSide {
    Ask,
    Bid,
}

/// A memory-efficient order type.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Order([u16; 4]);

impl Order {
    #[inline]
    pub(crate) fn is_invalid(&self) -> bool {
        self.0[0] == 0
    }

    #[inline]
    pub fn ask(memo: u16, qty: u16, price: u16) -> Self {
        Self([OrderSide::Ask as u8 as u16, memo, qty, price])
    }

    #[inline]
    pub fn bid(memo: u16, qty: u16, price: u16) -> Self {
        Self([OrderSide::Bid as u8 as u16, memo, qty, price])
    }

    #[inline]
    pub fn quantity(&self) -> u16 {
        let [_, _, q, _] = self.0;
        q
    }

    #[inline]
    pub fn quantity_mut(&mut self) -> &mut u16 {
        let [_, _, q, _] = &mut self.0;
        q
    }

    #[inline]
    pub fn price(&self) -> u16 {
        let [_, _, _, p] = self.0;
        p
    }

    #[inline]
    pub fn price_mut(&mut self) -> &mut u16 {
        let [_, _, _, p] = &mut self.0;
        p
    }

    #[inline]
    pub fn memo_mut(&mut self) -> &mut u16 {
        let [_, m, _, _] = &mut self.0;
        m
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u16; 4] {
        &self.0
    }

    #[inline]
    pub fn metadata(&self) -> OrderMetadata {
        OrderMetadata(self.0[0])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct OrderMetadata(u16);

pub const SIDE_BIT: u16 = 0b0000_0010;
pub const DEAD_BIT: u16 = 0b0000_0001;

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
