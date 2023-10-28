//! Trading module for the exchange, contains the orderbook and order matching logic.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::Asset;

pub mod orderbook;
pub use orderbook::{Order, OrderIndex, OrderSide, OrderType, Orderbook};

pub mod self_trade_protection;
pub use self_trade_protection::SelfTradeProtection;

pub mod timeinforce;
pub use timeinforce::TimeInForce;

pub mod pending_fill;
pub use pending_fill::{ExecutePendingFillError, FillType, PendingFill};

pub mod try_fill_order;
pub use try_fill_order::{try_fill_orders, TryFillOrdersError};

pub mod trigger;
pub use trigger::Triggers;

/// The unique identifier for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct OrderUuid(pub uuid::Uuid);
impl OrderUuid {
    fn new_v4() -> OrderUuid {
        OrderUuid(uuid::Uuid::new_v4())
    }
}

/// type-alias for a [`tokio::sync::mpsc::Sender``] that sends [TradingEngineCmd]s.
pub type TradingEngineTx = mpsc::Sender<TradingEngineCmd>;

/// type-alias for a [`tokio::sync::mpsc::Receiver``] that receives [TradingEngineCmd]s.
pub type TradingEngineRx = mpsc::Receiver<TradingEngineCmd>;

#[derive(Debug, Deserialize, Serialize)]
pub struct PlaceOrder {
    asset: Asset,
    user_uuid: uuid::Uuid,
    price: NonZeroU32,
    quantity: NonZeroU32,
    order_type: OrderType,
    stp: SelfTradeProtection,
    time_in_force: TimeInForce,
    side: OrderSide,
}

pub type PlaceOrderTx = oneshot::Sender<Result<PlaceOrderResult, TradingEngineError>>;

impl PlaceOrder {
    pub fn new(
        asset: Asset,
        user_uuid: uuid::Uuid,
        price: NonZeroU32,
        quantity: NonZeroU32,
        order_type: OrderType,
        stp: SelfTradeProtection,
        time_in_force: TimeInForce,
        side: OrderSide,
    ) -> Self {
        Self {
            asset,
            user_uuid,
            price,
            quantity,
            order_type,
            stp,
            time_in_force,
            side,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelOrder {
    user_uuid: uuid::Uuid,
    order_uuid: OrderUuid,
}

pub type CancelOrderTx = oneshot::Sender<Result<(), TradingEngineError>>;

impl CancelOrder {
    pub fn new(user_uuid: uuid::Uuid, order_uuid: OrderUuid) -> Self {
        Self {
            user_uuid,
            order_uuid,
        }
    }
}

#[derive(Debug, Error)]
pub enum PlaceOrderError {
    #[error("order was not completely filled due to insufficient liquidity")]
    FillOrKillFailed,
    #[error("order was not completely filled due to insufficient liquidity")]
    InsufficientLiquidity,
}

pub struct PlaceOrderResult {
    // original order information
    pub asset: Asset,
    pub user_uuid: uuid::Uuid,
    pub price: NonZeroU32,
    pub quantity: NonZeroU32,
    pub order_type: OrderType,
    pub stp: SelfTradeProtection,
    pub time_in_force: TimeInForce,
    pub side: OrderSide,
    // result of the order
    pub order_uuid: OrderUuid,
    pub order_index: Option<OrderIndex>,
    pub fill_type: FillType,
    pub quantity_filled: u32,
    pub quantity_remaining: u32,
}

pub fn do_place_order(
    assets: &mut Assets,
    PlaceOrder {
        asset,
        user_uuid,
        price,
        quantity,
        order_type,
        stp,
        time_in_force,
        side,
    }: PlaceOrder,
) -> Result<PlaceOrderResult, TradingEngineError> {
    let asset_book = assets.match_asset_mut(asset);

    let taker: Order = Order {
        memo: u32::MAX,
        quantity,
        price,
    };

    // use super::try_fill_order to create a pending fill and execute it.
    let pending_fill = try_fill_orders(asset_book.orderbook_mut(), taker, side, order_type)
        .expect("todo: handle error");

    match (pending_fill.taker_fill_outcome(), time_in_force) {
        (FillType::Complete, _) => (), // do nothing, order was completely filled.
        (FillType::Partial, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
        (FillType::Partial, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
        (FillType::Partial, TimeInForce::ImmediateOrCancel) => (), // commit the partial fill, but do not add to orderbook.
        (FillType::Partial, TimeInForce::FillOrKill) => {
            // there were no resting orders that could be filled against the taker order.
            return Err(PlaceOrderError::FillOrKillFailed.into());
        }
        (FillType::None, TimeInForce::GoodTilCanceled) => (), // add to orderbook as resting order.
        (FillType::None, TimeInForce::GoodTilDate) => (), // add to orderbook as resting order, it will be tracked and cancelled separately
        (FillType::None, TimeInForce::ImmediateOrCancel) => {
            // no fill, no orderbook entry, NO SOUP FOR YOU!
            return Err(PlaceOrderError::InsufficientLiquidity.into());
        }
        (FillType::None, TimeInForce::FillOrKill) => {
            return Err(PlaceOrderError::FillOrKillFailed.into())
        }
    }

    match pending_fill.commit() {
        Ok((fill_type, order)) => {
            if let Some(order) = order {
                let order_index = if matches!(time_in_force, TimeInForce::ImmediateOrCancel) {
                    // partial fill, but we do not add it to the orderbook because it is an IOC order.
                    None
                } else {
                    // order was not completely filled, add it to the orderbook.
                    Some(match side {
                        OrderSide::Buy => asset_book.orderbook_mut().push_bid(order),
                        OrderSide::Sell => asset_book.orderbook_mut().push_ask(order),
                    })
                };

                assert!(quantity.get() >= order.quantity.get());

                Ok(PlaceOrderResult {
                    asset,
                    user_uuid,
                    order_index,
                    price,
                    quantity,
                    order_type,
                    stp,
                    time_in_force,
                    side,
                    order_uuid: OrderUuid::new_v4(),
                    fill_type,
                    quantity_filled: quantity.get() - order.quantity.get(),
                    quantity_remaining: order.quantity.get(),
                })
            } else {
                todo!()
            }
        }
        Err(_) => todo!(),
    }
}

pub fn do_cancel_order(
    assets: &mut Assets,
    CancelOrder {
        user_uuid,
        order_uuid,
    }: CancelOrder,
) -> Result<(), TradingEngineError> {
    let (order_index, asset) = match assets.order_uuids.get(&order_uuid).cloned() {
        Some((a, b)) => (a, b),
        None => {
            return Err(TradingEngineError::OrderNotFound(user_uuid, order_uuid));
        }
    };

    let asset_book = assets.match_asset_mut(asset);

    asset_book
        .orderbook_mut()
        .remove(order_index)
        .expect("checked order");

    Ok(())
}

#[derive(Debug, Error)]
pub enum TradingEngineError {
    #[error("the trading engine is suspended")]
    Suspended,
    #[error("unserializable input to trading engine")]
    UnserializableInput,
    #[error("order not found for user {0:?} and order uuid {1:?}")]
    OrderNotFound(uuid::Uuid, OrderUuid),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("place order error")]
    PlaceOrder(#[from] PlaceOrderError),
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum TradeCmdPayload {
    PlaceOrder(PlaceOrder),
    CancelOrder(CancelOrder),
}

pub enum TradeCmd {
    /// place an order
    PlaceOrder((PlaceOrder, PlaceOrderTx)),
    /// cancel an order
    CancelOrder((CancelOrder, CancelOrderTx)),
}

/// enumeration of all the commands the trading engine can process.
pub enum TradingEngineCmd {
    /// a signal to shutdown the trading engine
    Shutdown,
    /// a trade command like placing an order or canceling an order.
    Trade(TradeCmd),
    /// a trade command deserialized from json used to initialize the trading engine.
    Bootstrap(TradeCmdPayload),
}
impl TradingEngineCmd {
    pub(crate) fn consume_respond_with_error(self, err: TradingEngineError) {
        if let Self::Trade(cmd) = self {
            match cmd {
                TradeCmd::PlaceOrder((_, tx)) => {
                    let _ = tx.send(Err(err));
                }
                TradeCmd::CancelOrder((_, tx)) => {
                    let _ = tx.send(Err(err));
                }
            };
        }
    }
}

/// the "state" of an asset book for a trading engine.
pub struct AssetBook {
    asset: Asset,
    orderbook: Orderbook,
}

impl AssetBook {
    pub fn new(asset: Asset) -> Self {
        Self {
            asset,
            orderbook: Orderbook::new(),
        }
    }

    pub fn orderbook_mut(&mut self) -> &mut Orderbook {
        &mut self.orderbook
    }
}

pub struct Assets {
    pub order_uuids: ahash::AHashMap<OrderUuid, (OrderIndex, Asset)>,
    pub eth: AssetBook,
    pub btc: AssetBook,
}

impl Assets {
    fn match_asset_mut(&mut self, asset: Asset) -> &mut AssetBook {
        match asset {
            Asset::Ether => &mut self.eth,
            Asset::Bitcoin => &mut self.btc,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::spawn_trading_engine::{spawn_trading_engine, SpawnTradingEngine};
    use crate::Config;

    use super::*;

    async fn trading_engine_fixture(db_pool: sqlx::PgPool) -> (Config, SpawnTradingEngine) {
        let config = crate::config::Config::load_from_toml("");
        let spawn_trading_engine = spawn_trading_engine(&config, db_pool).await;
        (config, spawn_trading_engine)
    }

    #[sqlx::test]
    async fn test_startup_shutdown(db_pool: sqlx::PgPool) {
        let (_config, spawn_trading_engine) = trading_engine_fixture(db_pool).await;
        spawn_trading_engine
            .input
            .send(TradingEngineCmd::Shutdown)
            .await
            .unwrap();
        spawn_trading_engine.handle.await.unwrap();
    }
}
