//! Module for placing orders in the matching engine.
//!
use super::asset_code::AssetCode;
use super::orderbook::OrderIndex;
use super::orderbook::TimeInForce;
use super::pending_fill::FillType;
use super::try_fill_order::try_fill_orders;
use crate::asset_pair::BaseQuote;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderData;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::orderbook::Orderbook;
use crate::orderbook::SelfTradeProtection;
use crate::orderflags::OrderFlags;
use crate::pending_fill::Fill;
use crate::pending_fill::PendingFill;
use crate::pending_fill::TifViolation;
use crate::price::Price;
use crate::reserve_money::ReserveByAssetError;
use common_core::web::middleware::clerk::ClerkUserId;

pub trait OrderDetails: Clone {
    fn order_side(&self) -> OrderSide;
    fn quantity(&self) -> Option<NonZeroDecimal>;
    fn price(&self) -> Price;
    fn order_type(&self) -> OrderType;
    fn time_in_force(&self) -> TimeInForce;
    fn stp(&self) -> SelfTradeProtection;
    /// For Iceberg orders, returns the display quantity (visible in the book) should return None for non-iceberg orders.
    fn display_quantity(&self) -> Option<NonZeroDecimal>;
    fn order_flags(&self) -> OrderFlags;
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceOrderArgs<D> {
    pub base_quote: BaseQuote,
    pub user_id: ClerkUserId,
    pub order_uuid: OrderUuid,
    pub details: D,
}

/// Error that can occur when placing an order.
#[derive(Debug, thiserror::Error)]
pub enum PlaceOrderError {
    /// Time-in-force violation
    #[error("time in force violation: {0}")]
    TifViolation(#[from] TifViolation),
    /// some asset pair (base/quote) was not found in the system
    #[error("invalid asset pair")]
    InvalidAssetPair,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MatchEvent {
    Trade {
        aggregate_filled_quantity: NonZeroDecimal,
        taker_order_id: OrderUuid,
        // taker_user_id: ClerkUserId,
        order_side: OrderSide,
        taker_fully_filled: bool,
        // timestamp: (),
        // trades: Vec<OrderIndex>,
    },
    Reduce {
        amount_reduced: NonZeroDecimal,
        order_removed: bool,
        order_price: NonZeroDecimal,
        order_id: OrderUuid,
        // user_id: ClerkUserId,
        // timestamp: ()
    },
    Reject {
        amount_rejected: NonZeroDecimal,
        price: NonZeroDecimal,
        order_id: OrderUuid,
        // user_id: ClerkUserId,
        // timestamp: ()
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PlaceOrderOk<D> {
    /// original order information
    pub args: PlaceOrderArgs<D>,
    /// events that were produced as a result of placing the order
    pub events: Vec<MatchEvent>,
}

/// Take a [`PendingFill`] and commit the difference back to the [`Orderbook`]
pub fn place_order<'ob, D>(
    pending_fill: PendingFill<'ob>,
    place_order_args: PlaceOrderArgs<D>,
) -> Result<PlaceOrderOk<D>, PlaceOrderError>
where
    D: OrderDetails,
{
    let PlaceOrderArgs {
        base_quote: _,
        user_id,
        order_uuid,
        details,
    } = &place_order_args;

    if let Err(tif_violation) = pending_fill.check_tif(details.time_in_force()) {
        pending_fill.abort();
        return Err(tif_violation.into());
    }

    let mut events: Vec<MatchEvent> = pending_fill
        .fills
        .iter()
        .map(|fill| {
            assert_ne!(fill.fill_type, FillType::NotFilled);

            let order_data = pending_fill
                .orderbook
                .get(fill.order_index)
                .expect("always exists");
            let amount_filled = if let FillType::Partial { amount_filled } = fill.fill_type {
                amount_filled
            } else if fill.fill_type == FillType::Complete {
                order_data.remaining_quantity
            } else {
                // Cancelled fills - use zero amount since no trade occurred
                crate::decimal::NonZeroDecimal::new(crate::decimal::Decimal::ONE).unwrap()
            };

            MatchEvent::Reduce {
                amount_reduced: amount_filled,
                order_removed: matches!(fill.fill_type, FillType::Complete),
                order_price: fill.order_index.price,
                order_id: order_data.order_id,
            }
        })
        .collect();

    match pending_fill.taker_fill_outcome {
        FillType::Complete => {
            events.push(MatchEvent::Trade {
                aggregate_filled_quantity: details.quantity().unwrap(),
                taker_order_id: place_order_args.order_uuid,
                order_side: details.order_side(),
                taker_fully_filled: true,
                // trades: pending_fill.fills,
            });
        }
        FillType::Partial { amount_filled } => {
            events.push(MatchEvent::Trade {
                aggregate_filled_quantity: amount_filled,
                taker_order_id: place_order_args.order_uuid,
                order_side: details.order_side(),
                taker_fully_filled: false,
                // trades: pending_fill.fills,
            });
        }
        FillType::NotFilled => (),
        FillType::Cancelled => {
            // Taker orders should never be cancelled by STP
            // This would indicate a logic error
            unreachable!("taker orders should not be cancelled by STP");
        }
    };

    let orderbook = pending_fill.commit();

    Ok(PlaceOrderOk {
        args: place_order_args,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_code::SymbolVocabulary;
    use crate::decimal::Decimal;
    use crate::order_uuid::OrderUuid;
    use crate::orderbook::Orderbook;
    use crate::try_fill_order::try_fill_orders;
    use std::str::FromStr;

    // Test helper struct that implements OrderDetails
    #[derive(Clone)]
    struct TestOrder {
        side: OrderSide,
        order_type: OrderType,
        price: Decimal,
        quantity: Decimal,
        time_in_force: TimeInForce,
        stp: SelfTradeProtection,
    }

    impl OrderDetails for TestOrder {
        fn order_side(&self) -> OrderSide {
            self.side
        }

        fn quantity(&self) -> Option<NonZeroDecimal> {
            NonZeroDecimal::new(self.quantity).ok()
        }

        fn price(&self) -> Price {
            Price {
                prefix: None,
                amount: self.price,
                is_percentage: false,
            }
        }

        fn order_type(&self) -> OrderType {
            self.order_type
        }

        fn time_in_force(&self) -> TimeInForce {
            self.time_in_force
        }

        fn stp(&self) -> SelfTradeProtection {
            self.stp
        }

        fn display_quantity(&self) -> Option<NonZeroDecimal> {
            None
        }

        fn order_flags(&self) -> OrderFlags {
            OrderFlags::default()
        }
    }

    #[test]
    fn test_ioc_partial_fill_succeeds() {
        let mut orderbook = Orderbook::new_empty();

        // Only 30 units available, but taker wants 50
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("test_user".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_id, order);
        };

        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("100").unwrap(),
            quantity: Decimal::from_str("50").unwrap(),
            time_in_force: TimeInForce::ImmediateOrCancel, // IOC allows partial fills
            stp: SelfTradeProtection::CancelBoth,
        };

        let pending_fill =
            try_fill_orders(&mut orderbook, &taker, taker.price.into(), None).unwrap();

        // Verify it's a partial fill
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            FillType::Partial { .. }
        ));

        // Create vocabulary and asset codes properly
        let vocab: SymbolVocabulary = vec!["BTC".to_string(), "USD".to_string()]
            .into_iter()
            .collect();
        let btc = AssetCode::from_str_and_vocabulary("BTC", &vocab).unwrap();
        let usd = AssetCode::from_str_and_vocabulary("USD", &vocab).unwrap();

        let args = PlaceOrderArgs {
            base_quote: (btc, usd),
            user_id: ClerkUserId("test_user".to_string()),
            order_uuid: OrderUuid(uuid::Uuid::new_v4()),
            details: taker,
        };

        // IOC should succeed with partial fill
        let result = place_order(pending_fill, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fok_fails_on_partial_fill() {
        let mut orderbook = Orderbook::new_empty();

        // Only 30 units available, but taker wants 50
        {
            let orderbook: &mut Orderbook = &mut orderbook;
            let price = Decimal::from_str("100").unwrap();
            let quantity = Decimal::from_str("30").unwrap();
            let order_id = OrderUuid(uuid::Uuid::new_v4());
            let user_id = common_core::web::middleware::clerk::ClerkUserId("test_user".to_string());
            let order = TestOrder {
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                price,
                quantity,
                time_in_force: TimeInForce::GoodTilCanceled,
                stp: SelfTradeProtection::CancelNewest,
            };
            let price_nz = NonZeroDecimal::new(price).unwrap();
            orderbook.insert(price_nz, order_id, user_id, order);
        };

        let taker = TestOrder {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Decimal::from_str("100").unwrap(),
            quantity: Decimal::from_str("50").unwrap(),
            time_in_force: TimeInForce::FillOrKill, // FOK requires complete fill
            stp: SelfTradeProtection::CancelBoth,
        };

        let pending_fill =
            try_fill_orders(&mut orderbook, &taker, taker.price.into(), None).unwrap();

        // Verify it's a partial fill
        assert!(matches!(
            pending_fill.taker_fill_outcome,
            FillType::Partial { .. }
        ));

        // Create vocabulary and asset codes properly
        let vocab: SymbolVocabulary = vec!["BTC".to_string(), "USD".to_string()]
            .into_iter()
            .collect();
        let btc = AssetCode::from_str_and_vocabulary("BTC", &vocab).unwrap();
        let usd = AssetCode::from_str_and_vocabulary("USD", &vocab).unwrap();

        let args = PlaceOrderArgs {
            base_quote: (btc, usd),
            user_id: ClerkUserId("test_user".to_string()),
            order_uuid: OrderUuid(uuid::Uuid::new_v4()),
            details: taker,
        };

        // FOK should fail on partial fill with TifViolation error
        let result = place_order(pending_fill, args);
        assert!(matches!(
            result,
            Err(PlaceOrderError::TifViolation(TifViolation::FillOrKill))
        ));
    }
}
