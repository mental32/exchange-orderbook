use std::num::NonZeroU32;

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
#[repr(transparent)]
pub struct OrderMetadata(u16);

pub const SIDE_BIT: u16 = 0b0000_0010;
pub const DEAD_BIT: u16 = 0b0000_0001;

impl OrderMetadata {
    #[inline]
    pub fn side(&self) -> OrderSide {
        let side = self.0 & SIDE_BIT;

        match side {
            0 => OrderSide::Sell,
            1 => OrderSide::Buy,
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
