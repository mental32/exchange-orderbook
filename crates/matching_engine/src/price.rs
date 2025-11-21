use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PricePrefix {
    /// relative "adds the amount to"
    Plus,
    /// relative "subtracts the amount from the last traded price"
    Sub,
    /// relative "hashes the amount with the last traded price"
    Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Price {
    pub prefix: Option<PricePrefix>,
    pub amount: Decimal,
    pub is_percentage: bool,
}

impl FromStr for Price {
    type Err = ();

    fn from_str(st: &str) -> Result<Self, Self::Err> {
        use chumsky::prelude::*;
        use chumsky::text::newline;

        let decimal = any::<_, extra::Err<EmptyErr>>()
            .and_is(newline().or(just("%").ignored()).not())
            .repeated()
            .at_least(1)
            .to_slice()
            .try_map(|substr: &str, _| substr.parse::<Decimal>().map_err(|_| EmptyErr::default()));

        just("+")
            .map(|_| PricePrefix::Plus)
            .or(just("-").map(|_| PricePrefix::Sub))
            .or(just("#").map(|_| PricePrefix::Hash))
            .or_not()
            .then(decimal)
            .then(just("%").or_not())
            .try_map(|((prefix, amount), percentage), _| {
                if prefix.is_none() && percentage.is_some() {
                    return Err(EmptyErr::default());
                }

                Ok(Price {
                    prefix,
                    amount,
                    is_percentage: percentage.is_some(),
                })
            })
            .parse(st)
            .into_output()
            .ok_or(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
#[error("Invalid price")]
pub struct InvalidPrice;

impl Price {
    pub fn compute_to_decimal(
        &self,
        last_traded_price: NonZeroDecimal,
        order_side: OrderSide,
        order_type: OrderType,
    ) -> Result<NonZeroDecimal, InvalidPrice> {
        if self.prefix.is_none() && self.is_percentage {
            // Percentage without prefix is invalid
            return Err(InvalidPrice);
        }

        let amount_delta = self.amount; // may be zero (valid for percentage) or negative (valid for absolute)

        if let Some(PricePrefix::Sub) = self.prefix
            && self.is_percentage
            && amount_delta.abs() >= Decimal::ONE_HUNDRED
        {
            // "-100%" or greater would make price zero/negative
            return Err(InvalidPrice);
        }

        let rv = match (self.is_percentage, self.prefix) {
            (true, None) => {
                // Percentage without prefix is invalid
                return Err(InvalidPrice);
            }
            (true, Some(PricePrefix::Plus)) => {
                *last_traded_price
                    + (*last_traded_price * amount_delta / crate::decimal::Decimal::ONE_HUNDRED)
            } // "+5%" = base + (base * 5%)
            (true, Some(PricePrefix::Sub)) => {
                *last_traded_price
                    - (*last_traded_price * amount_delta / crate::decimal::Decimal::ONE_HUNDRED)
            } // "-5%" = base - (base * 5%)
            (true, Some(PricePrefix::Hash)) => {
                let offset =
                    *last_traded_price * amount_delta / crate::decimal::Decimal::ONE_HUNDRED; // "#5%" = base * (5%)
                match (order_side, order_type) {
                    // Limit/Iceberg: Buy adds offset, Sell subtracts offset
                    (OrderSide::Buy, OrderType::Limit | OrderType::Iceberg) => {
                        *last_traded_price + offset
                    }
                    (OrderSide::Sell, OrderType::Limit | OrderType::Iceberg) => {
                        *last_traded_price - offset
                    }

                    // Stop-loss types: Same direction as limit
                    (
                        OrderSide::Buy,
                        OrderType::StopLoss
                        | OrderType::StopLossLimit
                        | OrderType::TrailingStop
                        | OrderType::TrailingStopLimit,
                    ) => *last_traded_price + offset,
                    (
                        OrderSide::Sell,
                        OrderType::StopLoss
                        | OrderType::StopLossLimit
                        | OrderType::TrailingStop
                        | OrderType::TrailingStopLimit,
                    ) => *last_traded_price - offset,

                    // Take-profit: INVERTED direction (close winning positions)
                    (OrderSide::Buy, OrderType::TakeProfit | OrderType::TakeProfitLimit) => {
                        *last_traded_price - offset
                    }
                    (OrderSide::Sell, OrderType::TakeProfit | OrderType::TakeProfitLimit) => {
                        *last_traded_price + offset
                    }

                    // Market orders: # prefix is meaningless
                    (_, OrderType::Market) => return Err(InvalidPrice),
                }
            }
            (false, None) => {
                // Absolute price "50000"
                amount_delta
            }
            (false, Some(PricePrefix::Plus)) => *last_traded_price + amount_delta,
            (false, Some(PricePrefix::Sub)) => *last_traded_price - amount_delta,
            (false, Some(PricePrefix::Hash)) => match (order_side, order_type) {
                // Limit/Iceberg: Buy adds amount, Sell subtracts amount
                (OrderSide::Buy, OrderType::Limit | OrderType::Iceberg) => {
                    *last_traded_price + amount_delta
                }
                (OrderSide::Sell, OrderType::Limit | OrderType::Iceberg) => {
                    *last_traded_price - amount_delta
                }

                // Stop-loss types: Same direction as limit
                (
                    OrderSide::Buy,
                    OrderType::StopLoss
                    | OrderType::StopLossLimit
                    | OrderType::TrailingStop
                    | OrderType::TrailingStopLimit,
                ) => *last_traded_price + amount_delta,
                (
                    OrderSide::Sell,
                    OrderType::StopLoss
                    | OrderType::StopLossLimit
                    | OrderType::TrailingStop
                    | OrderType::TrailingStopLimit,
                ) => *last_traded_price - amount_delta,

                // Take-profit: INVERTED direction (close winning positions)
                (OrderSide::Buy, OrderType::TakeProfit | OrderType::TakeProfitLimit) => {
                    *last_traded_price - amount_delta
                }
                (OrderSide::Sell, OrderType::TakeProfit | OrderType::TakeProfitLimit) => {
                    *last_traded_price + amount_delta
                }

                // Market orders: # prefix is meaningless
                (_, OrderType::Market) => return Err(InvalidPrice),
            },
        };

        NonZeroDecimal::new(rv).map_err(|()| InvalidPrice)
    }

    pub fn is_relative(&self) -> bool {
        self.prefix.is_some()
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.prefix {
            Some(PricePrefix::Plus) => "+",
            Some(PricePrefix::Sub) => "-",
            Some(PricePrefix::Hash) => "#",
            None => "",
        };

        let amount = self.amount.to_string();

        let suffix = if self.is_percentage { "%" } else { "" };

        write!(f, "{prefix}{amount}{suffix}")
    }
}

#[cfg(feature = "serde")]
pub fn serialize<S>(value: &Price, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize as _;

    value.to_string().serialize(serializer)
}

#[cfg(feature = "serde")]
pub fn deserialize<'de, D>(deserializer: D) -> Result<Price, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let st: String = String::deserialize(deserializer)?;

    Price::from_str(&st).map_err(|_| serde::de::Error::custom("Invalid price format"))
}

#[cfg(feature = "serde")]
pub fn deserialize_price_option<'de, D>(deserializer: D) -> Result<Option<Price>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct PriceVisitor;

    impl<'de> serde::de::Visitor<'de> for PriceVisitor {
        type Value = Option<Price>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a price string or null")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value.parse() {
                Ok(price) => Ok(Some(price)),
                Err(()) => Err(E::custom("Invalid price format")),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(PriceVisitor)
}

#[cfg(feature = "serde")]
pub fn serialize_price_option<S>(value: &Option<Price>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(price) => crate::price::serialize(price, serializer),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod test {
    use crate::decimal::NonZeroDecimal;
    use crate::orderbook::OrderSide;
    use crate::orderbook::OrderType;
    use crate::price::Price;
    use crate::price::PricePrefix;
    use std::str::FromStr as _;

    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    struct Shim {
        #[cfg_attr(feature = "serde", serde(with = "crate::price"))]
        price: Price,
    }

    #[test]
    fn test_price_from_str() {
        assert_eq!(
            Price::from_str("+123.45%"),
            Ok(Price {
                prefix: Some(PricePrefix::Plus),
                amount: "123.45".parse().unwrap(),
                is_percentage: true,
            })
        );
    }

    #[cfg_attr(feature = "serde", test)]
    fn test_de_price() {
        let json = r#"{"price": "+123.45%"}"#;
        let shim: Shim = serde_json::from_str(json).unwrap();
        assert_eq!(
            shim.price,
            Price {
                prefix: Some(PricePrefix::Plus),
                amount: "123.45".parse().unwrap(),
                is_percentage: true,
            }
        );
    }

    #[cfg_attr(feature = "serde", test)]
    fn test_ser_price() {
        let price = Shim {
            price: Price {
                prefix: Some(PricePrefix::Plus),
                amount: "123.45".parse().unwrap(),
                is_percentage: true,
            },
        };
        let json = serde_json::to_string(&price).unwrap();
        assert_eq!(json, r#"{"price":"+123.45%"}"#);
    }

    #[test]
    fn test_resolve_absolute_price() {
        // Absolute price "50000" should return 50000 regardless of last price
        let price = Price {
            prefix: None,
            amount: "50000".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("48000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "50000".parse().unwrap());
    }

    #[test]
    fn test_resolve_plus_absolute() {
        // "+1000" = last + 1000
        let price = Price {
            prefix: Some(PricePrefix::Plus),
            amount: "1000".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "51000".parse().unwrap());
    }

    #[test]
    fn test_resolve_sub_absolute() {
        // "-1000" = last - 1000
        let price = Price {
            prefix: Some(PricePrefix::Sub),
            amount: "1000".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "49000".parse().unwrap());
    }

    #[test]
    fn test_resolve_plus_percentage() {
        // "+5%" = last + (last * 5%)
        // 50000 + (50000 * 0.05) = 52500
        let price = Price {
            prefix: Some(PricePrefix::Plus),
            amount: "5".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "52500".parse().unwrap());
    }

    #[test]
    fn test_resolve_sub_percentage() {
        // "-10%" = last - (last * 10%)
        // 50000 - (50000 * 0.10) = 45000
        let price = Price {
            prefix: Some(PricePrefix::Sub),
            amount: "10".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "45000".parse().unwrap());
    }

    #[test]
    fn test_hash_limit_buy_absolute() {
        // Limit Buy #1000 = last + 1000
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "1000".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "51000".parse().unwrap());
    }

    #[test]
    fn test_hash_limit_sell_absolute() {
        // Limit Sell #1000 = last - 1000
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "1000".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Sell, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "49000".parse().unwrap());
    }

    #[test]
    fn test_hash_limit_buy_percentage() {
        // Limit Buy #2% = last + (last * 2%)
        // 50000 + 1000 = 51000
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "2".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "51000".parse().unwrap());
    }

    #[test]
    fn test_hash_limit_sell_percentage() {
        // Limit Sell #2% = last - (last * 2%)
        // 50000 - 1000 = 49000
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "2".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Sell, OrderType::Limit)
            .unwrap();
        assert_eq!(*result, "49000".parse().unwrap());
    }

    #[test]
    fn test_hash_stoploss_buy() {
        // StopLoss Buy #500 = last + 500 (same as Limit)
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "500".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::StopLoss)
            .unwrap();
        assert_eq!(*result, "50500".parse().unwrap());
    }

    #[test]
    fn test_hash_stoploss_sell() {
        // StopLoss Sell #500 = last - 500 (same as Limit)
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "500".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Sell, OrderType::StopLoss)
            .unwrap();
        assert_eq!(*result, "49500".parse().unwrap());
    }

    #[test]
    fn test_hash_takeprofit_buy_inverted() {
        // TakeProfit Buy #500 = last - 500 (INVERTED from Limit!)
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "500".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::TakeProfit)
            .unwrap();
        assert_eq!(*result, "49500".parse().unwrap()); // Note: SUBTRACTED, not added
    }

    #[test]
    fn test_hash_takeprofit_sell_inverted() {
        // TakeProfit Sell #500 = last + 500 (INVERTED from Limit!)
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "500".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Sell, OrderType::TakeProfit)
            .unwrap();
        assert_eq!(*result, "50500".parse().unwrap()); // Note: ADDED, not subtracted
    }

    #[test]
    fn test_hash_takeprofit_limit_buy_inverted() {
        // TakeProfitLimit Buy #1% = last - (last * 1%)
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "1".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::TakeProfitLimit)
            .unwrap();
        assert_eq!(*result, "49500".parse().unwrap()); // 50000 - 500
    }

    #[test]
    fn test_hash_takeprofit_limit_sell_inverted() {
        // TakeProfitLimit Sell #1% = last + (last * 1%)
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "1".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Sell, OrderType::TakeProfitLimit)
            .unwrap();
        assert_eq!(*result, "50500".parse().unwrap()); // 50000 + 500
    }

    #[test]
    fn test_hash_iceberg_buy() {
        // Iceberg Buy #100 = last + 100
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "100".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::Iceberg)
            .unwrap();
        assert_eq!(*result, "50100".parse().unwrap());
    }

    #[test]
    fn test_hash_trailing_stop_buy() {
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "200".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price
            .compute_to_decimal(last_price, OrderSide::Buy, OrderType::TrailingStop)
            .unwrap();
        assert_eq!(*result, "50200".parse().unwrap());
    }

    #[test]
    fn test_hash_market_order_errors() {
        // Market orders with # prefix should error
        let price = Price {
            prefix: Some(PricePrefix::Hash),
            amount: "100".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price.compute_to_decimal(last_price, OrderSide::Buy, OrderType::Market);
        assert!(result.is_err(), "Market order with # prefix should error");
    }

    #[test]
    fn test_percentage_without_prefix_errors() {
        // "5%" without +/- prefix should error
        let price = Price {
            prefix: None,
            amount: "5".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price.compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit);
        assert!(result.is_err(), "Percentage without prefix should error");
    }

    #[test]
    fn test_sub_100_percent_errors() {
        // "-100%" would make price zero, should error
        let price = Price {
            prefix: Some(PricePrefix::Sub),
            amount: "100".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price.compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit);
        assert!(result.is_err(), "-100% should error (results in zero)");
    }

    #[test]
    fn test_sub_150_percent_errors() {
        // "-150%" would make price negative, should error
        let price = Price {
            prefix: Some(PricePrefix::Sub),
            amount: "150".parse().unwrap(),
            is_percentage: true,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price.compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit);
        assert!(result.is_err(), "-150% should error (results in negative)");
    }

    #[test]
    fn test_sub_amount_larger_than_price_errors() {
        // "-60000" when last price is 50000 would be negative
        let price = Price {
            prefix: Some(PricePrefix::Sub),
            amount: "60000".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price.compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit);
        assert!(
            result.is_err(),
            "Subtraction resulting in negative should error"
        );
    }

    #[test]
    fn test_zero_price_errors() {
        // Absolute price of 0 should error
        let price = Price {
            prefix: None,
            amount: "0".parse().unwrap(),
            is_percentage: false,
        };
        let last_price = NonZeroDecimal::new("50000".parse().unwrap()).unwrap();
        let result = price.compute_to_decimal(last_price, OrderSide::Buy, OrderType::Limit);
        assert!(result.is_err(), "Zero price should error");
    }
}
