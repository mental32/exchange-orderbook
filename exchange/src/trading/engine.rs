use std::num::NonZeroU32;

use tokio::sync::{mpsc, oneshot};

use crate::trading::{OrderSide, OrderType, Orderbook, SelfTradeProtection};
use crate::Asset;

use super::OrderUuid;

pub enum TradingEngineCmd {
    /// a signal to shutdown the trading engine
    Shutdown,
    /// a signal to suspend the trading engine, reject all messages until a Resume.
    Suspend,
    /// a signal to resume the trading engine, accept all messages until a Suspend.
    Resume,
    /// place an order
    PlaceOrder {
        asset: Asset,
        user_uuid: uuid::Uuid,
        price: NonZeroU32,
        quantity: NonZeroU32,
        order_type: OrderType,
        stp: SelfTradeProtection,
        side: OrderSide,
        response: oneshot::Sender<Result<OrderUuid, ()>>,
    },
    /// cancel an order
    CancelOrder {
        user_uuid: uuid::Uuid,
        order_uuid: OrderUuid,
        response: oneshot::Sender<Result<(), ()>>,
    },
    /// cancel all orders for a user
    CancelAllOrders {
        user_uuid: uuid::Uuid,
        response: oneshot::Sender<Result<(), ()>>,
    },
}

pub struct TradingEngineLoopInput {
    rx: mpsc::Receiver<TradingEngineCmd>,
    state: TradingEngineState,
}

impl TradingEngineLoopInput {
    pub fn new(rx: mpsc::Receiver<TradingEngineCmd>) -> Self {
        Self {
            rx,
            state: TradingEngineState {
                btc: AssetEngine {
                    asset: Asset::Bitcoin,
                    orderbook: Orderbook::new(),
                },
                eth: AssetEngine {
                    asset: Asset::Ether,
                    orderbook: Orderbook::new(),
                },
            },
        }
    }
}

pub struct AssetEngine {
    asset: Asset,
    orderbook: Orderbook,
}

pub struct TradingEngineState {
    btc: AssetEngine,
    eth: AssetEngine,
}

/// execute commands for the trading engine.
pub fn trading_engine_loop(input: TradingEngineLoopInput) {
    use TradingEngineCmd as T;

    let TradingEngineLoopInput { mut rx, state } = input;

    while let Some(cmd) = rx.blocking_recv() {
        match cmd {
            T::Suspend => todo!(),
            T::Resume => todo!(),
            T::Shutdown => break,
            T::PlaceOrder {
                asset,
                user_uuid,
                order_type,
                stp,
                side,
                response,
                price,
                quantity,
            } => todo!(),
            T::CancelOrder {
                user_uuid,
                order_uuid,
                response,
            } => todo!(),
            T::CancelAllOrders {
                user_uuid,
                response,
            } => todo!(),
        }
    }
}
