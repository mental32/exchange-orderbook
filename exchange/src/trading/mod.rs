//! Trading module for the exchange, contains the orderbook and order matching logic.

pub mod order;
pub use order::{Order, OrderMetadata, OrderSide, OrderType};

pub mod orderbook;
pub use orderbook::{OrderIndex, Orderbook};

pub mod price_level;

pub mod self_trade_protection;
pub use self_trade_protection::SelfTradeProtection;

pub mod timeinforce;
pub use timeinforce::TimeInForce;

pub mod pending_fill;
pub use pending_fill::{ExecutePendingFillError, FillType, PendingFill};

pub mod engine;
pub use engine::{spawn_trading_engine, TradingEngineCmd, TradingEngineRx};

pub mod try_fill_order;
pub(self) use try_fill_order::{try_fill_orders, TryFillOrdersError};

pub mod trigger;
pub use trigger::Triggers;

/// The unique identifier for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct OrderUuid(uuid::Uuid);

impl OrderUuid {
    /// Create a new random `OrderUuid` should only be used for testing purposes.
    fn new_v4() -> OrderUuid {
        OrderUuid(uuid::Uuid::new_v4())
    }
}
