//! small wrapper around [`rust_decimal`]
//!

use std::ops::Deref;

/// local alias for decimal type we use... should be [`sqlx::types::Decimal`] (aka [`rust_decimal::Decimal`])
pub type Decimal = sqlx::types::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct NonZeroDecimal(Decimal);

impl From<Decimal> for NonZeroDecimal {
    #[track_caller]
    fn from(value: Decimal) -> Self {
        Self::new(value).expect("decimal value must be positive")
    }
}

impl Deref for NonZeroDecimal {
    type Target = Decimal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NonZeroDecimal {
    pub const ONE: Self = Self(Decimal::ONE);
    pub const ONE_HUNDRED: Self = Self(Decimal::ONE_HUNDRED);
    pub const MAX: Self = Self(Decimal::MAX);

    pub fn new(value: Decimal) -> Result<Self, ()> {
        if value <= Decimal::ZERO {
            Err(())
        } else {
            Ok(Self(value))
        }
    }

    pub fn write(&mut self, value: Decimal) -> Result<(), ()> {
        if value <= Decimal::ZERO {
            Err(())
        } else {
            self.0 = value;
            Ok(())
        }
    }
}
