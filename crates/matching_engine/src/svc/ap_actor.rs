//! Actor pattern for a single asset pair (ap) orderbook
//!
//! Each asset pair, like BTC/USD, has its own actor that processes messages sequentially
//! and maintains its own state (the orderbook)

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::AssetPairRow;
use crate::orderbook::OrderSide;
use crate::orderbook::Orderbook;
use crate::reserve_money::reserve_by_asset;
use crate::svc::routes::trade_add_order::TradeAddOrder;
use common_core::money38_18::Money38_18;
use std::ops::Deref;
use std::ops::DerefMut;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MessageIn {
    PlaceOrder(crate::place_order::PlaceOrderArgs<TradeAddOrder>),
    CancelOrder(crate::cancel_order::CancelOrder),
    Suspend,
    Resume,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum MessageOut {
    PlaceOrder(crate::place_order::PlaceOrderResult<TradeAddOrder>),
    CancelOrder(Option<crate::orderbook::Order>),
    Suspend,
    Resume,
    Shutdown,
}

pub type Response = Result<Box<[MessageOut]>, Error>;

pub type Envelope = (oneshot::Sender<Response>, Vec<MessageIn>);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("message serialization error")]
    UnserializableInput {
        base_quote: (AssetCode, AssetCode),
        message: MessageIn,
        #[source]
        error: serde_json::Error,
    },
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("order placement error")]
    PlaceOrder(#[from] crate::place_order::PlaceOrderError),
    #[error("asset-pair processor is suspended; mesages will not be processed")]
    Suspended,
}

struct Inner {
    rcv: mpsc::Receiver<Envelope>,
    pg_pool: sqlx::PgPool,
    running: bool,
    orderbook: Orderbook,
    asset_pair: AssetPairRow,
    symbol_vocabulary: SymbolVocabulary,
}

impl Inner {
    pub async fn switch_msg(&mut self, msg: MessageIn) -> Result<MessageOut, Error> {
        match msg {
            MessageIn::PlaceOrder(args) => {
                let pending_fill =
                    crate::try_fill_order::try_fill_orders(&mut self.orderbook, &args.details)
                        .unwrap();

                if args.details.validate_only {
                    return Ok(MessageOut::PlaceOrder(todo!("")));
                }

                let quantity_money = Money38_18(args.details.quantity);
                let (base, quote) = args.base_quote;

                let reserve_money = reserve_by_asset(
                    self.pg_pool.clone(),
                    args.user_id.clone(),
                    match args.details.side {
                        OrderSide::Buy => todo!("for buys it needs to be quantity x price"),
                        OrderSide::Sell => quantity_money.clone(),
                    },
                    match args.details.side {
                        OrderSide::Buy => quote, // to buy e.g. BTC/USD we reserve the quote asset (x btc for y usd)
                        OrderSide::Sell => base, // to sell we reserve the base asset (x btc for y usd)
                    },
                    &self.symbol_vocabulary,
                )
                .await
                .expect("todo: map_err funds reserve to Error type");

                tracing::trace!(?reserve_money.previous_balance, ?reserve_money.new_balance, "reserved funds");

                let revert_on_drop = reserve_money
                    .defer_revert(tokio::runtime::Handle::current(), self.pg_pool.clone());

                let res = crate::place_order::do_place_order(pending_fill, args)?;

                revert_on_drop.cancel();

                Ok(MessageOut::PlaceOrder(todo!()))
            }
            MessageIn::CancelOrder(co) => {
                let maybe_order = crate::cancel_order::do_cancel_order(&mut self.orderbook, co);
                Ok(MessageOut::CancelOrder(maybe_order))
            }
            MessageIn::Suspend => {
                self.running = false;
                Ok(MessageOut::Suspend)
            }
            MessageIn::Resume => {
                self.running = true;
                Ok(MessageOut::Resume)
            }
            MessageIn::Shutdown => {
                // handle shutdown
                Ok(MessageOut::Shutdown)
            }
        }
    }
}

pub async fn asset_processor_loop(
    rcv: mpsc::Receiver<Envelope>,
    pg_pool: sqlx::PgPool,
    asset_pair: AssetPairRow,
    symbol_vocabulary: SymbolVocabulary,
) {
    let mut proc = Inner {
        rcv,
        pg_pool,
        running: true,
        orderbook: Orderbook::new_empty(),
        asset_pair,
        symbol_vocabulary: symbol_vocabulary.clone(),
    };

    let base_quote = proc
        .asset_pair
        .base_quote(&symbol_vocabulary)
        .expect("must be resolvable");

    'outer: loop {
        match proc.rcv.recv().await {
            Some((snd, batch)) => {
                if !proc.running {
                    tracing::warn!(?batch, "processor is suspended");
                    let _ = snd.send(Err(Error::Suspended));
                    continue;
                }

                let mut output = vec![];

                let mut tx: sqlx::Transaction<'static, sqlx::Postgres> = proc
                    .pg_pool
                    .begin()
                    .await
                    .expect("Failed to start transaction");

                for msg in batch {
                    let res = match serde_json::to_value((base_quote, &msg)) {
                        Ok(jstr) => {
                            match sqlx::query!(
                                "INSERT INTO t_trading_event_source (jstr) VALUES ($1)",
                                jstr
                            )
                            .execute(tx.deref_mut())
                            .await
                            {
                                Ok(_) => {
                                    let res: Result<_, Error> = proc.switch_msg(msg).await;
                                    res
                                }
                                Err(e) => Err(Error::Database(e)),
                            }
                        }
                        Err(error) => Err(Error::UnserializableInput {
                            base_quote: base_quote.clone(),
                            message: msg,
                            error,
                        }),
                    };

                    match res {
                        Ok(val) => output.push(val),
                        Err(err) => {
                            tracing::error!(?err, "switch_msg failed");
                            tx.rollback().await.unwrap();
                            let _ = snd.send(Err(err));
                            continue 'outer;
                        }
                    };
                }

                match snd.send(Ok(output.into_boxed_slice())) {
                    Err(_) => {
                        tracing::error!(
                            "processor could not respond to message sender; the sender was dropped"
                        );
                        tx.rollback().await.unwrap();
                    }
                    Ok(()) => {
                        tx.commit().await.unwrap();
                    }
                }
            }
            None => {
                tracing::warn!("asset pair actor message channel closed");
                break;
            }
        }
    }
}
