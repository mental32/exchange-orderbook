use std::num::NonZeroU32;

use tokio::sync::{mpsc, oneshot};

use crate::trading::{FillType, Order, OrderSide, OrderType, Orderbook, SelfTradeProtection};
use crate::Asset;

use super::{OrderIndex, OrderUuid, Triggers};

/// type-alias for a [`tokio::sync::mpsc::Sender``] that sends [TradingEngineCmd]s.
pub type TradingEngineTx = mpsc::Sender<TradingEngineCmd>;

/// type-alias for a [`tokio::sync::mpsc::Receiver``] that receives [TradingEngineCmd]s.
pub type TradingEngineRx = mpsc::Receiver<TradingEngineCmd>;

pub struct PlaceOrder {
    asset: Asset,
    user_uuid: uuid::Uuid,
    price: NonZeroU32,
    quantity: NonZeroU32,
    order_type: OrderType,
    stp: SelfTradeProtection,
    side: OrderSide,
    response: oneshot::Sender<Result<OrderUuid, TradingEngineError>>,
}

pub struct CancelOrder {
    user_uuid: uuid::Uuid,
    order_uuid: OrderUuid,
    response: oneshot::Sender<Result<(), TradingEngineError>>,
}

pub struct CancelAllOrders {
    user_uuid: uuid::Uuid,
    response: oneshot::Sender<Result<(), TradingEngineError>>,
}

#[derive(Debug, thiserror::Error)]
pub enum TradingEngineError {
    #[error("the trading engine is suspended")]
    Suspended,
}

/// enumeration of all the commands the trading engine can process.
pub enum TradingEngineCmd {
    /// a signal to shutdown the trading engine
    Shutdown,
    /// a signal to suspend the trading engine, reject all messages until a Resume.
    Suspend,
    /// a signal to resume the trading engine, accept all messages until a Suspend.
    Resume,
    /// place an order
    PlaceOrder(PlaceOrder),
    /// cancel an order
    CancelOrder(CancelOrder),
    /// cancel all orders for a user
    CancelAllOrders(CancelAllOrders),
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
            Self::PlaceOrder(PlaceOrder { response, .. }) => inner(response, err),
            Self::CancelOrder(CancelOrder { response, .. }) => inner(response, err),
            Self::CancelAllOrders(CancelAllOrders { response, .. }) => inner(response, err),
        }
    }
}

/// the "input" to the [trading_engine_loop] function.
///
/// this is a struct that contains the [TradingEngineRx] and the [Assets] that the trading engine will operate on.
///
struct TradingEngineLoopIn {
    /// the receiver for the trading engine commands.
    rx: TradingEngineRx,
    /// the "state" of the trading engine see [Assets].
    assets: Assets,
    /// the database connection pool.
    triggers: Triggers,
}

impl TradingEngineLoopIn {
    pub fn new(rx: TradingEngineRx, assets: Assets, triggers: Triggers) -> Self {
        Self {
            rx,
            assets,
            triggers,
        }
    }
}

/// the "state" of an asset book for a trading engine.
pub struct AssetBook {
    asset: Asset,
    orderbook: Orderbook,
    order_uuid_to_order_index: ahash::AHashMap<OrderUuid, OrderIndex>,
}

/// the "state" of all asset books for a trading engine.
#[non_exhaustive]
pub struct Assets {
    /// the asset book for bitcoin
    btc: AssetBook,
    /// the asset book for ether
    eth: AssetBook,
}
impl Assets {
    /// enable all assets, this is somewhat of a hack for now as it does not account for multiple replicas of the trading engine each with a different set of enabled assets.
    pub(crate) fn all() -> Assets {
        Assets {
            btc: AssetBook {
                asset: Asset::Bitcoin,
                orderbook: Orderbook::new(),
                order_uuid_to_order_index: ahash::AHashMap::new(),
            },
            eth: AssetBook {
                asset: Asset::Ether,
                orderbook: Orderbook::new(),
                order_uuid_to_order_index: ahash::AHashMap::new(),
            },
        }
    }

    /// returns a reference to the asset book for the given asset.
    pub fn match_asset_mut(&mut self, asset: Asset) -> &mut AssetBook {
        match asset {
            Asset::Bitcoin => &mut self.btc,
            Asset::Ether => &mut self.eth,
        }
    }
}

fn trading_engine_loop(input: TradingEngineLoopIn) {
    use TradingEngineCmd as T;

    let TradingEngineLoopIn {
        mut rx,
        mut assets,
        triggers: db_pool,
    } = input;

    // implementation note regarding rx.blocking_recv():
    //
    //  this trading engine event loop is designed to run on its own thread.
    //  but the input to the system is running in a tokio runtime,
    //  luckily, the channel type we are using is capable of being used in a
    // blocking mode on threads where there is no tokio runtime running.
    //
    //  within the guts of rx.blocking_recv(), it will use a standard approach
    // of parking the thread until a signal is "sent" to it by setting
    // an atomic integer type to a value and then waking the thread to process
    // futures that are ready to be processed in a block_on call. this includes
    // the async rx.recv call

    let mut is_suspended = false;

    while let Some(cmd) = rx.blocking_recv() {
        if is_suspended {
            match cmd {
                T::Resume => is_suspended = false,
                _ => continue,
            }
        }

        match cmd {
            T::Suspend => {
                is_suspended = true;
            }
            T::Resume => {
                is_suspended = false;
            }
            T::Shutdown => break,
            T::PlaceOrder(place_order) => {
                let asset_book = assets.match_asset_mut(place_order.asset);

                let taker: Order = Order {
                    memo: u32::MAX,
                    quantity: place_order.quantity,
                    price: place_order.price,
                };

                let taker_side = place_order.side;
                let order_type = place_order.order_type;

                // use super::try_fill_order to create a pending fill and execute it.
                let pending_fill = super::try_fill_orders(
                    &mut asset_book.orderbook,
                    taker,
                    taker_side,
                    order_type,
                )
                .expect("todo: handle error");

                match pending_fill.commit() {
                    Ok((fill_type, order)) => {
                        if let Some(order) = order {
                            // order was not completely filled, add it to the orderbook.
                            let order_ix = match taker_side {
                                OrderSide::Buy => asset_book.orderbook.push_bid(order),
                                OrderSide::Sell => asset_book.orderbook.push_ask(order),
                            };

                            // asset_book
                            //     .order_uuid_to_order_index
                            //     .insert(order_uuid, order_ix);

                            todo!()
                        }
                    }
                    Err(_) => todo!(),
                };

                let _ = place_order.response.send(Ok(OrderUuid::new_v4()));
            }
            T::CancelOrder(cancel_order) => {
                todo!();
            }
            T::CancelAllOrders(cancel_all_orders) => {
                todo!();
            }
        }
    }
}

pub type SpawnTradingEngineOut = (mpsc::Sender<TradingEngineCmd>, std::thread::JoinHandle<()>);

/// spawns a new thread and returns a [TradingEngineTx] and a [std::thread::JoinHandle] for the thread.
///
/// the [TradingEngineTx] can be used to send commands to the trading engine. the [std::thread::JoinHandle] can be used to wait for the thread to exit.
///
/// the [te_msg_chan_cap] parameter is the capacity of the channel used to send messages to the trading engine. this is the maximum number of messages that can be queued up before the sender blocks.
///
pub fn spawn_trading_engine(te_msg_chan_cap: usize) -> SpawnTradingEngineOut {
    let (te_tx, rx) = tokio::sync::mpsc::channel(te_msg_chan_cap);

    let te_handle = std::thread::Builder::new()
        .name("trading-engine".to_string())
        .stack_size(8_000_000) // 8MB otherwise we get a stack overflow on OSX as non-main threads have a 512KB stack size.
        .spawn(move || {
            let te_input = TradingEngineLoopIn::new(rx, Assets::all(), Default::default());
            trading_engine_loop(te_input)
        })
        .unwrap();

    (te_tx, te_handle)
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
