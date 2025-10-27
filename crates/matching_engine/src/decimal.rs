//! small wrapper around [`rust_decimal`]
//!

use std::ops::Deref;

pub use rust_decimal::dec;
pub use rust_decimal::serde;

/// local alias for decimal type we use... should be [`sqlx::types::Decimal`] (aka [`rust_decimal::Decimal`])
pub type Decimal = sqlx::types::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
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

#[cfg(feature = "serde")]
pub fn deserialize_decimal_option<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: ::serde::Deserializer<'de>,
{
    struct DecimalVisitor;

    impl<'de> ::serde::de::Visitor<'de> for DecimalVisitor {
        type Value = Option<Decimal>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a decimal string or null")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: ::serde::de::Error,
        {
            match value.parse::<rust_decimal::Decimal>() {
                Ok(decimal) => Ok(Some(decimal.into())),
                Err(_) => Err(E::custom("Invalid decimal format")),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(DecimalVisitor)
}

#[cfg(feature = "serde")]
pub fn serialize_decimal_option<S>(
    value: &Option<Decimal>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ::serde::Serializer,
{
    match value {
        Some(decimal) => rust_decimal::serde::str::serialize(decimal, serializer),
        None => serializer.serialize_none(),
    }
}
