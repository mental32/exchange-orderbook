//! small wrapper around [`rust_decimal`]
//!

/// local alias for decimal type we use... should be [`sqlx::types::Decimal`] (aka [`rust_decimal::Decimal`])
pub type Decimal = sqlx::types::Decimal;
