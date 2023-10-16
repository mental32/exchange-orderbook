use std::num::NonZeroU32;

use tokio::sync::{mpsc, oneshot};

use crate::trading::{OrderSide, OrderType, Orderbook, SelfTradeProtection};

use super::OrderUuid;

enum TradingEngineCmd {
    PlaceOrder {
        user_uuid: uuid::Uuid,
        price: NonZeroU32,
        quantity: NonZeroU32,
        order_type: OrderType,
        stp: SelfTradeProtection,
        side: OrderSide,
        response: oneshot::Sender<Result<OrderUuid, ()>>,
    },
    CancelOrder {
        user_uuid: uuid::Uuid,
        order_uuid: OrderUuid,
        response: oneshot::Sender<Result<(), ()>>,
    },
    CancelAllOrders {
        user_uuid: uuid::Uuid,
        response: oneshot::Sender<Result<(), ()>>,
    },
}

/// The input to the [`trading_engine_loop`] function.
struct TradingEngineLoopInput {
    cmd_rx: mpsc::Receiver<TradingEngineCmd>,
    state: TradingEngineState,
}

/// The state of the trading engine.
struct TradingEngineState {
    orderbook: Orderbook,
}

/// execute commands for the trading engine, it is one trading engine per asset.
fn trading_engine_loop(input: TradingEngineLoopInput) {
    use TradingEngineCmd as T;

    let TradingEngineLoopInput { mut cmd_rx, state } = input;

    while let Some(cmd) = cmd_rx.blocking_recv() {
        match cmd {
            T::PlaceOrder {
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
