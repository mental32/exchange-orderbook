//! "processor" for a single asset pair (ap) orderbook using Actor pattern
//!
//! Each asset pair, like BTC/USD, gets a task spawned to process messages
//! as commands which control or modify the orderbook.
//!
//! Relevant types when using this module (the interface):
//! - [`MessageIn`]
//! - [`MessageOut`]
//! - [`Error`]
//! - [`Response`]
//! - [`Envelope`]
//!

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::AssetPairRow;
use crate::cancel_order::CancelOrder;
use crate::decimal::NonZeroDecimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderIndex;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::orderbook::Orderbook;
use crate::pending_fill::Fill;
use crate::pending_fill::FillType;
use crate::place_order::PlaceOrderArgs;
use crate::place_order::PlaceOrderError;
use crate::place_order::PlaceOrderOk;
use crate::price::Price;
use crate::price::PricePrefix;
use crate::reserve_money::ReserveByAssetError;
use crate::reserve_money::reserve_by_asset;
use crate::svc::routes::trade_add_order::TradeAddOrder;
use crate::try_fill_order::TryFillOrdersError;
use common_core::money38_18::Money38_18;
use std::ops::Deref;
use std::ops::DerefMut;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MsgIn {
    PlaceOrder(PlaceOrderArgs<TradeAddOrder>),
    CancelOrder(CancelOrder),
    Suspend,
    Resume,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum MsgOut {
    PlacedOrder {},
    ValidatedOrder,
    CancelOrder(Option<()>),
    WillSuspend,
    WillResume,
    WillShutdown,
}

pub type Response = Result<MsgOut, Error>;

pub type Envelope = (oneshot::Sender<Response>, MsgIn);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("message serialization error")]
    UnserializableInput {
        base_quote: (AssetCode, AssetCode),
        message: MsgIn,
        #[source]
        error: serde_json::Error,
    },
    #[error("due to a database error the message-in was not inserted ({0})")]
    DbFailedToInsertMsgIn(sqlx::Error),
    #[error("order placement error")]
    PlaceOrder(#[from] PlaceOrderError),
    #[error("no reference price to place order using relative price")]
    NoReferencePrice,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("asset-pair processor is suspended; mesages will not be processed")]
    ProcessorIsSuspended,
    #[error("invalid price: computed price is zero or negative")]
    InvalidPrice,
}

// data that gets modified within switch_msg and might need to be rolled back
#[derive(Debug, Clone)]
struct InnerVariable {
    last_traded_price: Option<NonZeroDecimal>,
    is_suspended: bool,
    order_uuids: ahash::AHashMap<OrderUuid, OrderIndex>,
}

// data that does not change or is too big to copy (the orderbook)
struct InnerStable {
    pg_pool: sqlx::PgPool,
    orderbook: Orderbook,
    asset_pair: AssetPairRow,
    symbol_vocabulary: SymbolVocabulary,
}

impl InnerStable {
    pub async fn switch_msg(
        &mut self,
        msg_in: MsgIn,
        vars: &mut InnerVariable,
    ) -> Result<MsgOut, Error> {
        match msg_in {
            MsgIn::PlaceOrder(args) => {
                let resolved_price: NonZeroDecimal =
                    if let Some(last_traded_price) = vars.last_traded_price {
                        args.details
                            .price
                            .resolve_to_absolute(
                                last_traded_price,
                                args.details.side,
                                args.details.order_type,
                            )
                            .map_err(|_| Error::InvalidPrice)?
                    } else {
                        if args.details.price.is_relative() {
                            return Err(Error::NoReferencePrice);
                        } else {
                            NonZeroDecimal::new(args.details.price.amount)
                                .map_err(|()| Error::InvalidPrice)?
                        }
                    };

                let pending_fill = match crate::try_fill_order::try_fill_orders(
                    &mut self.orderbook,
                    &args.details,
                    resolved_price,
                    Some(&args.user_id),
                ) {
                    Ok(f) => f,
                    Err(TryFillOrdersError::ZeroQuantity) => todo!(),
                };

                let quantity_money = Money38_18(args.details.quantity);
                let money38_18 = match args.details.side {
                    OrderSide::Buy => {
                        if args.details.order_flags.volume_in_quote_currency {
                            assert_eq!(
                                args.details.order_type,
                                OrderType::Market,
                                "only valid for market buys"
                            );
                            quantity_money
                        } else if args.details.order_type == OrderType::Market {
                            // quantity is in base (BTC), need to multiply by price
                            // Market orders: simulate fill to calculate cost
                            todo!("calculate max cost from pending_fill")
                        } else {
                            // Limit orders: quantity × price
                            assert_ne!(
                                args.details.order_type,
                                OrderType::Market,
                                "only valid for non market buys"
                            );
                            Money38_18(args.details.quantity * resolved_price.deref())
                        }
                    }
                    OrderSide::Sell => quantity_money.clone(),
                };

                if args.details.validate_only {
                    return Ok(MsgOut::ValidatedOrder);
                }

                let reserve_money = match reserve_by_asset(
                    self.pg_pool.clone(),
                    args.user_id.clone(),
                    money38_18,
                    {
                        let (base, quote) = args.base_quote;
                        match args.details.side {
                            OrderSide::Buy => quote, // to buy e.g. BTC/USD we reserve the quote asset (x btc for y usd)
                            OrderSide::Sell => base, // to sell we reserve the base asset (x btc for y usd)
                        }
                    },
                    &self.symbol_vocabulary,
                )
                .await
                {
                    Ok(r) => r,
                    Err(ReserveByAssetError::InsufficientFunds) => {
                        return Err(Error::InsufficientFunds);
                    }
                    Err(ReserveByAssetError::Database(sqlx_error)) => todo!(),
                };

                tracing::trace!(?reserve_money.previous_balance, ?reserve_money.new_balance, "reserved funds");

                let revert_on_drop = reserve_money
                    .defer_revert(tokio::runtime::Handle::current(), self.pg_pool.clone());

                let new_trade_price = pending_fill
                    .fills
                    .iter()
                    .filter(|fill| fill.fill_type != FillType::Cancelled)
                    .map(|fill| fill.order_index.price)
                    .last();

                let PlaceOrderOk { args, events } =
                    match crate::place_order::place_order(pending_fill, args) {
                        Ok(PlaceOrderOk { args, events }) => PlaceOrderOk { args, events },
                        Err(PlaceOrderError::TifViolation(_)) => todo!(),
                        Err(PlaceOrderError::InvalidAssetPair) => todo!(),
                    };

                vars.last_traded_price = new_trade_price;

                revert_on_drop.cancel();

                Ok(MsgOut::PlacedOrder {})
            }
            MsgIn::CancelOrder(co) => {
                let maybe_order = crate::cancel_order::do_cancel_order(&mut self.orderbook, co);
                Ok(MsgOut::CancelOrder(None))
            }
            MsgIn::Suspend => {
                vars.is_suspended = true;
                Ok(MsgOut::WillSuspend)
            }
            MsgIn::Resume => {
                vars.is_suspended = false;
                Ok(MsgOut::WillResume)
            }
            MsgIn::Shutdown => {
                vars.is_suspended = true;
                Ok(MsgOut::WillShutdown)
            }
        }
    }
}

pub async fn asset_processor_loop(
    mut rcv: mpsc::Receiver<Envelope>,
    pg_pool: sqlx::PgPool,
    asset_pair: AssetPairRow,
    symbol_vocabulary: SymbolVocabulary,
) {
    let mut proc = InnerStable {
        pg_pool,
        orderbook: Orderbook::new_empty(),
        asset_pair,
        symbol_vocabulary,
    };

    let mut vars = InnerVariable {
        order_uuids: Default::default(),
        is_suspended: false,
        last_traded_price: None, // TODO: REPLACE THIS WITH AN INDEX PRICE INSTEAD OF NONE
    };

    let base_quote = proc
        .asset_pair
        .base_quote(&proc.symbol_vocabulary)
        .expect("symbols are always resolvable");

    'rcv: loop {
        let (snd, msg_in) = rcv
            .recv()
            .await
            .expect("asset pair actor message channel closed");

        if vars.is_suspended {
            let _ = snd.send(Err(Error::ProcessorIsSuspended));
            continue;
        }

        let mut will_shutdown = false;
        let checkpoint = vars.clone();
        let mut tx: sqlx::Transaction<'static, sqlx::Postgres> = proc
            .pg_pool
            .begin()
            .await
            .expect("Failed to start transaction");

        let result: Result<MsgOut, Error> = match serde_json::to_value((base_quote, &msg_in)) {
            Ok(json_value) => {
                match sqlx::query!(
                    "INSERT INTO t_trading_event_source (jstr) VALUES ($1)",
                    json_value
                )
                .execute(tx.deref_mut())
                .await
                {
                    Ok(_) => proc.switch_msg(msg_in, &mut vars).await,
                    Err(sqlx_error) => Err(Error::DbFailedToInsertMsgIn(sqlx_error)),
                }
            }
            Err(error) => Err(Error::UnserializableInput {
                base_quote: base_quote.clone(),
                message: msg_in,
                error,
            }),
        };

        match result {
            Ok(msg_out) => {
                if let MsgOut::WillShutdown = msg_out {
                    will_shutdown = true;
                }

                match snd.send(Ok(msg_out)) {
                    Err(_msg_out) => {
                        tracing::error!(
                            "processor could not respond to message sender; the sender was dropped"
                        );
                        vars = checkpoint;
                        tx.rollback().await.expect("could not rollback transaction");
                    }
                    Ok(()) => {
                        tx.commit().await.expect("could not commit transaction");
                    }
                }
            }
            Err(err) => {
                tracing::error!(?err, "switch_msg failed");
                vars = checkpoint;
                tx.rollback().await.expect("could not rollback transaction");
                let _ = snd.send(Err(err));
                continue 'rcv;
            }
        };

        if will_shutdown {
            return;
        }
    }
}
