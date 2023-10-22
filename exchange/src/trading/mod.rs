//! Trading module for the exchange, contains the orderbook and order matching logic.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

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
pub struct OrderUuid(uuid::Uuid);

impl OrderUuid {
    /// Create a new random `OrderUuid` should only be used for testing purposes.
    fn new_v4() -> OrderUuid {
        OrderUuid(uuid::Uuid::new_v4())
    }
}

use crate::Asset;

/// type-alias for a [`tokio::sync::mpsc::Sender``] that sends [TradingEngineCmd]s.
pub type TradingEngineTx = mpsc::Sender<TradingEngineCmd>;

/// type-alias for a [`tokio::sync::mpsc::Receiver``] that receives [TradingEngineCmd]s.
pub type TradingEngineRx = mpsc::Receiver<TradingEngineCmd>;

fn default_oneshot_channel<T>() -> oneshot::Sender<Result<T, TradingEngineError>> {
    oneshot::channel().0
}

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
    #[serde(skip, default = "default_oneshot_channel")]
    response: oneshot::Sender<Result<OrderUuid, TradingEngineError>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelOrder {
    user_uuid: uuid::Uuid,
    order_uuid: OrderUuid,
    #[serde(skip, default = "default_oneshot_channel")]
    response: oneshot::Sender<Result<(), TradingEngineError>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelAllOrders {
    user_uuid: uuid::Uuid,
    #[serde(skip, default = "default_oneshot_channel")]
    response: oneshot::Sender<Result<(), TradingEngineError>>,
}

#[derive(Debug, thiserror::Error)]
pub enum TradingEngineError {
    #[error("the trading engine is suspended")]
    Suspended,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum TradeCmd {
    /// place an order
    PlaceOrder(PlaceOrder),
    /// cancel an order
    CancelOrder(CancelOrder),
    /// cancel all orders for a user
    CancelAllOrders(CancelAllOrders),
}

/// enumeration of all the commands the trading engine can process.
pub enum TradingEngineCmd {
    /// a signal to shutdown the trading engine
    Shutdown,
    /// a signal to suspend the trading engine, reject all messages until a Resume.
    Suspend,
    /// a signal to resume the trading engine, accept all messages until a Suspend.
    Resume,
    ///
    Trade(TradeCmd),
}

impl TradingEngineCmd {
    pub fn consume_respond_with_error(self, err: TradingEngineError) {
        fn inner<T>(tx: oneshot::Sender<Result<T, TradingEngineError>>, err: TradingEngineError) {
            if let Err(t) = tx.send(Err(err)) {}
        }

        match self {
            Self::Shutdown => (),
            Self::Suspend => (),
            Self::Resume => (),
            Self::Trade(this) => match this {
                TradeCmd::PlaceOrder(PlaceOrder { response, .. }) => inner(response, err),
                TradeCmd::CancelOrder(CancelOrder { response, .. }) => inner(response, err),
                TradeCmd::CancelAllOrders(CancelAllOrders { response, .. }) => inner(response, err),
            },
        }
    }
}

/// the "state" of an asset book for a trading engine.
pub struct AssetBook {
    asset: Asset,
    orderbook: Orderbook,
    order_uuid_to_order_index: ahash::AHashMap<OrderUuid, OrderIndex>,
}

impl AssetBook {
    pub fn new(asset: Asset) -> Self {
        Self {
            asset,
            orderbook: Orderbook::new(),
            order_uuid_to_order_index: ahash::AHashMap::new(),
        }
    }

    pub fn orderbook_mut(&mut self) -> &mut Orderbook {
        &mut self.orderbook
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pos_engine_shutdown() {
        let (te_tx, te_handle) = spawn_trading_engine(1);
        te_tx.send(TradingEngineCmd::Shutdown).await.unwrap();
        te_handle.join().unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_place_order() {
        let (te_tx, _) = spawn_trading_engine(5);
        let (response, res_rx) = oneshot::channel();

        // Test placing a valid order
        let valid_order = PlaceOrder {
            asset: Asset::Bitcoin,
            user_uuid: uuid::Uuid::new_v4(),
            price: NonZeroU32::new(10000).unwrap(),
            quantity: NonZeroU32::new(1).unwrap(),
            order_type: OrderType::Limit,
            stp: SelfTradeProtection::CancelOldest,
            time_in_force: TimeInForce::GoodTilCanceled,
            side: OrderSide::Buy,
            response,
        };

        te_tx
            .send(TradingEngineCmd::PlaceOrder(valid_order))
            .await
            .unwrap();

        let order_uuid = res_rx.await.unwrap().unwrap();
    }
}

pub struct Assets {
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

pub(crate) fn trading_engine_step(cmd: TradeCmd, assets: &mut Assets) {
    use TradeCmd as T;

    match cmd {
        T::PlaceOrder(PlaceOrder {
            asset,
            quantity,
            price,
            side,
            order_type,
            response,
            time_in_force,
            ..
        }) => {
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
                (FillType::Partial, TimeInForce::GoodTilCanceled) => todo!(),
                (FillType::Partial, TimeInForce::GoodTilDate) => todo!(),
                (FillType::Partial, TimeInForce::ImmediateOrCancel) => todo!(),
                (FillType::Partial, TimeInForce::FillOrKill) => todo!(),
                // there were no resting orders that could be filled against the taker order.
                (FillType::None, TimeInForce::GoodTilCanceled) => todo!(),
                (FillType::None, TimeInForce::GoodTilDate) => todo!(),
                (FillType::None, TimeInForce::ImmediateOrCancel) => todo!(),
                (FillType::None, TimeInForce::FillOrKill) => todo!(),
            }

            match pending_fill.commit() {
                Ok((fill_type, order)) => {
                    if let Some(order) = order {
                        // order was not completely filled, add it to the orderbook.
                        let order_ix = match side {
                            OrderSide::Buy => asset_book.orderbook_mut().push_bid(order),
                            OrderSide::Sell => asset_book.orderbook_mut().push_ask(order),
                        };

                        todo!()
                    }
                }
                Err(_) => todo!(),
            };

            // let _ = response.send(Ok(OrderUuid::new_v4()));
        }
        T::CancelOrder(cancel_order) => {
            todo!();
        }
        T::CancelAllOrders(cancel_all_orders) => {
            todo!();
        }
    }
}
