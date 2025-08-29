// If you need array support later, implement PgHasArrayType for Postgres arrays.
use std::str::FromStr;

use rust_decimal::Decimal;
use sqlx::Decode;
use sqlx::Encode;
use sqlx::Postgres;
use sqlx::Type;
use sqlx::postgres::PgArgumentBuffer;
use sqlx::postgres::PgTypeInfo;

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use serde::Serialize;

/// Newtype for the Postgres domain `money38_18` (DECIMAL(38,18)).
///
/// This delegates encoding/decoding to `rust_decimal::Decimal` so it can be
/// used directly with `sqlx` queries as a parameter and a returned column.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Money38_18(pub Decimal);

impl From<Decimal> for Money38_18 {
    fn from(d: Decimal) -> Self {
        Money38_18(d)
    }
}

impl From<Money38_18> for Decimal {
    fn from(m: Money38_18) -> Self {
        m.0
    }
}

impl std::fmt::Display for Money38_18 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Money38_18 {
    type Err = rust_decimal::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Decimal::from_str(s).map(Money38_18)
    }
}

// --- sqlx traits ---------------------------------------------------------

impl Type<Postgres> for Money38_18 {
    fn type_info() -> PgTypeInfo {
        // Must match the domain/type name in Postgres
        PgTypeInfo::with_name("money38_18")
    }
}

impl<'r> Decode<'r, Postgres> for Money38_18 {
    fn decode(
        value: <Postgres as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        // Delegate to Decimal's Decode implementation
        let dec = <Decimal as Decode<Postgres>>::decode(value)?;
        Ok(Money38_18(dec))
    }
}

impl<'q> Encode<'q, Postgres> for Money38_18 {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> sqlx::encode::IsNull {
        <Decimal as Encode<Postgres>>::encode_by_ref(&self.0, buf)
    }
}
