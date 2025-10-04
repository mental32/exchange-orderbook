use crate::decimal::Decimal;
use crate::orderbook::OrderType;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ConditionalParameters {
    pub order_type: OrderType,
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub limit_price: Decimal,
    pub limit_price_type: String,
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    pub trigger_price: Decimal,
    pub trigger_price_type: String,
}
