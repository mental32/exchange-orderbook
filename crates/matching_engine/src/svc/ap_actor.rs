//! "processor" for a single asset pair (ap) orderbook using Actor pattern
//!
//! Each asset pair, like BTC/USD, gets a task spawned to process messages
//! as commands which control or modify the orderbook.
//!
//! Relevant types when using this module (the interface):
//! - [`MsgIn`]
//! - [`MsgOut`]
//! - [`Error`]
//! - [`Response`]
//! - [`Envelope`]
//!

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::AssetPairRow;
use crate::asset_pair::BaseQuote;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::order_uuid::OrderUuid;
use crate::orderbook::OrderData;
use crate::orderbook::OrderIndex;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::orderbook::Orderbook;
use crate::pending_fill::Fill;
use crate::pending_fill::FillType;
use crate::pending_fill::OrderDetails;
use crate::pending_fill::TifViolation;
use crate::price::Price;
use crate::price::PricePrefix;
use crate::reserve_money::ReserveByAssetError;
use crate::reserve_money::reserve_money_by_asset;
use crate::svc::order_management::CancelOrderBy;
use crate::svc::order_management::OpenOrder;
use crate::svc::order_management::PlaceOrderArgs;
use crate::svc::order_management::UserProfile;
use crate::svc::routes::trade_add_order::TradeAddOrder;
use crate::svc::routes::trade_cancel_order::TradeCancelOrder;
use crate::try_fill_order::TryFillOrdersError;
use ahash::HashMap;
use anyhow::Context as _;
use common_core::money38_18::Money38_18;
use common_core::web::middleware::clerk::ClerkUserId;
use futures::StreamExt as _;
use itertools::Itertools as _;
use std::ops::Deref;
use std::ops::DerefMut;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tracing::Instrument as _;

use super::order_management::Tracking;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MsgIn {
    PlaceOrder(PlaceOrderArgs<TradeAddOrder>),
    CancelOrderBy {
        user_id: ClerkUserId,
        cancel_order_by: CancelOrderBy,
    },
    Suspend,
    Resume,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum MsgOut {
    OrderPlaced,
    OrderValidated,
    OrderCancelled {
        success: Vec<OrderUuid>,
        failed: Vec<OrderUuid>,
    },
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
        #[cfg(feature = "serde")]
        #[source]
        serde_json_error: serde_json::Error,
    },
    #[error("message-in could not be persisted to event table: {0}")]
    CouldNotPersistToEventSource(sqlx::Error),
    #[error("time in force violation: {0}")]
    TifViolation(#[from] TifViolation),
    #[error("placing order with zero quantity in order details")]
    ZeroQuantity,
    #[error("no reference price to place order using relative price")]
    NoReferencePrice,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("asset-pair processor is suspended; mesages will not be processed")]
    ProcessorIsSuspended,
    #[error("invalid price: computed price is zero or negative")]
    InvalidPrice,
    #[error("order was cancelled")]
    OrderCancelled,
    #[error("no open positions for user")]
    NoOpenPositions,
    #[error("order not found")]
    OrderNotFound,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BroadcastEvent {}

// subset of data in the processor that we want to clone
#[derive(Clone)]
struct Vars {
    last_traded_price: Option<NonZeroDecimal>,
    is_suspended: bool,
    will_shutdown: bool,
}

// data that does not change or is too big to copy (e.g. the orderbook)
pub(super) struct ApState {
    pg_pool: sqlx::PgPool,
    orderbook: Orderbook,
    tracking: Tracking,
    asset_pair_row: AssetPairRow,
    symbol_vocabulary: SymbolVocabulary,
    expiry_queue: Vec<(u64, OrderUuid, Option<OrderIndex>)>,
}

impl ApState {
    async fn handle_expiry(&mut self, vars: &mut Vars) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut expired = vec![];
        while let Some((ts, _, _)) = self.expiry_queue.first() {
            if *ts <= now {
                expired.push(self.expiry_queue.remove(0));
            } else {
                break;
            }
        }

        if expired.is_empty() {
            return;
        }

        let mut transaction: sqlx::Transaction<'static, sqlx::Postgres> = self
            .pg_pool
            .begin()
            .await
            .expect("Failed to start transaction");

        let base_quote = self
            .asset_pair_row
            .base_quote(&self.symbol_vocabulary)
            .expect("symbols are always resolvable");
        let (base, quote) = base_quote;

        for (_, order_uuid, order_index_opt) in expired {
            if let Some(order_index) = order_index_opt {
                let order_data = self
                    .orderbook
                    .get(order_index)
                    .expect("order exists in book");

                let (currency, reserved_amount) = match order_index.side {
                    OrderSide::Buy => {
                        let amount = *order_data.remaining_quantity * *order_data.price;
                        (quote, Money38_18(amount))
                    }
                    OrderSide::Sell => {
                        let amount = *order_data.remaining_quantity;
                        (base, Money38_18(amount))
                    }
                };

                let user_source_id = format!("user:{}", order_data.user_id.0);
                let currency_str = currency.as_str(&self.symbol_vocabulary);

                sqlx::query!(
                    r#"
                    INSERT INTO t_account_tx_journal
                    (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                    VALUES (
                        (SELECT id FROM t_money_accounts WHERE source_type = 'user' AND source_id = $1 AND currency = $2),
                        (SELECT id FROM t_money_accounts WHERE source_type != 'user' AND currency = $2),
                        $2,
                        $3::numeric,
                        $4,
                        $5
                    )
                    "#,
                    user_source_id,
                    currency_str,
                    reserved_amount.0,
                    "gtd_expiry_refund",
                    uuid::Uuid::new_v4().to_string()
                )
                .execute(transaction.deref_mut())
                .await
                .expect("refund transaction succeeds");

                self.orderbook
                    .remove(order_index)
                    .expect("order exists in book");
            }

            self.tracking.send_modify(|map| {
                for (_, profile) in map.iter_mut() {
                    profile.open_orders.retain(|o| o.order_uuid != order_uuid);
                }
            });
        }

        transaction
            .commit()
            .await
            .expect("expiry transaction commits");
    }

    async fn switch_msg_in(
        &mut self,
        msg_in: MsgIn,
        snd: oneshot::Sender<Response>,
        vars: &mut Vars,
    ) {
        let fut = async move {
            let vars_copy_pre = vars.clone();

            let mut transaction: sqlx::Transaction<'static, sqlx::Postgres> = self
                .pg_pool
                .begin()
                .await
                .expect("Failed to start transaction");

            let base_quote = self
                .asset_pair_row
                .base_quote(&self.symbol_vocabulary)
                .expect("symbols are always resolvable");

            #[cfg(not(feature = "serde"))]
            return Error(Error::CouldNotPersistToEventSource); // XXX: i dont like this

            #[cfg(feature = "serde")]
            {
                let value = serde_json::to_value((
                    self.asset_pair_row.base_quote(&self.symbol_vocabulary),
                    &msg_in,
                ))
                .map_err(|err| Error::UnserializableInput {
                    base_quote: base_quote.clone(),
                    message: msg_in.clone(),
                    serde_json_error: err,
                })?;

                let pg_query_result = sqlx::query!(
                    "INSERT INTO t_trading_event_source (jstr) VALUES ($1)",
                    value
                )
                .execute(transaction.deref_mut())
                .await
                .map_err(|err| Error::CouldNotPersistToEventSource(err))?;

                assert_eq!(pg_query_result.rows_affected(), 1);
            }

            match msg_in {
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
                    vars.will_shutdown = true;
                    Ok(MsgOut::WillShutdown)
                }

                _ if vars.is_suspended => Err(Error::ProcessorIsSuspended),

                MsgIn::PlaceOrder(mut args) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    if let Some(expiry_ts) = args.order_details.expiry_time.to_absolute_timestamp(now) {
                        args.order_details.time_in_force = crate::orderbook::TimeInForce::GoodTilDate(expiry_ts);
                    }

                    let computed_price: NonZeroDecimal =
                        if let Some(last_traded_price) = vars.last_traded_price {
                            args.order_details
                                .price()
                                .compute_to_decimal(
                                    last_traded_price,
                                    args.order_details.side,
                                    args.order_details.order_type,
                                )
                                .map_err(|_| Error::InvalidPrice)?
                        } else {
                            if args.order_details.price().is_relative() {
                                return Err(Error::NoReferencePrice);
                            } else {
                                NonZeroDecimal::new(args.order_details.price().amount)
                                    .map_err(|()| Error::InvalidPrice)?
                            }
                        };

                    let pending_fill = crate::try_fill_order::try_fill_orders(
                        &mut self.orderbook,
                        &args.order_details,
                        computed_price,
                        Some(&args.user_id),
                    )
                    .map_err(|TryFillOrdersError::ZeroQuantity| Error::ZeroQuantity)?;

                    if let Err(tif_violation) = pending_fill.has_tif_violation(
                        args.order_details.order_type(),
                        args.order_details.time_in_force(),
                    ) {
                        pending_fill.abort();
                        if let Err(error) = transaction.rollback().await {
                            tracing::error!(?error, "database transaction rollback error");
                        }
                        return Err(Error::TifViolation(tif_violation));
                    }

                    if let Some(FillType::Cancelled) = pending_fill.taker_fill_outcome
                        && args.order_details.validate_only.unwrap_or(false)
                    {
                        return Err(Error::OrderCancelled);
                    }

                    let quantity_money =
                        Money38_18(args.order_details.quantity().unwrap().deref().clone());
                    let money38_18 = match args.order_details.side {
                        OrderSide::Buy => {
                            if args.order_details.order_flags().volume_in_quote_currency {
                                assert_eq!(
                                    args.order_details.order_type,
                                    OrderType::Market,
                                    "only valid for market buys"
                                );
                                quantity_money
                            } else if args.order_details.order_type == OrderType::Market {
                                let (filled_quantity, filled_cost) = pending_fill
                                    .fills
                                    .iter()
                                    .filter(|fill| fill.fill_type != FillType::Cancelled)
                                    .fold(
                                        (Decimal::ZERO, Decimal::ZERO),
                                        |(qty_acc, cost_acc), fill| {
                                            let fill_qty = match fill.fill_type {
                                                FillType::Complete { quantity } => *quantity,
                                                FillType::Partial { amount_filled } => {
                                                    *amount_filled
                                                }
                                                FillType::Cancelled => Decimal::ZERO,
                                            };

                                            (
                                                qty_acc + fill_qty,
                                                cost_acc
                                                    + (fill_qty * fill.order_index.price.deref()),
                                            )
                                        },
                                    );

                                Money38_18(filled_cost)
                            } else {
                                // Limit orders: quantity × price
                                // XXX: do we have to consider filled cost + resting portion?
                                assert_ne!(
                                    args.order_details.order_type,
                                    OrderType::Market,
                                    "only valid for non market buys"
                                );
                                Money38_18(
                                    args.order_details.quantity().unwrap().deref()
                                        * computed_price.deref(),
                                )
                            }
                        }
                        OrderSide::Sell => quantity_money.clone(),
                    };

                    if args.order_details.validate_only.unwrap_or(false) {
                        return Ok(MsgOut::OrderValidated);
                    }

                    let currency = {
                        let (base, quote) = args.base_quote;
                        match args.order_details.side {
                            OrderSide::Buy => quote, // to buy e.g. BTC/USD we reserve the quote asset (x btc for y usd)
                            OrderSide::Sell => base, // to sell we reserve the base asset (x btc for y usd)
                        }
                    };

                    // settlement: take funds from taker account
                    let reserve_money = reserve_money_by_asset(
                        transaction.deref_mut(),
                        args.user_id.clone(),
                        money38_18,
                        currency,
                        &self.symbol_vocabulary,
                    )
                    .await
                    .map_err(|err| match err {
                        ReserveByAssetError::InsufficientFunds => Error::InsufficientFunds,
                        ReserveByAssetError::Database(sqlx_error) => {
                            Error::CouldNotPersistToEventSource(sqlx_error)
                        }
                    })?;

                    tracing::trace!(?reserve_money.previous_balance, ?reserve_money.new_balance, "reserved funds");

                    // settlement: credit the accounts of filled orders
                    for Fill {
                        order_index,
                        fill_type,
                    } in &pending_fill.fills
                    {
                        let maker_order_data = pending_fill
                            .orderbook
                            .get(*order_index)
                            .expect("maker order always exists in orderbook during settlement");

                        // Extract fill amount based on FillType
                        let fill_amount = match fill_type {
                            FillType::Complete { quantity } => *quantity,
                            FillType::Partial { amount_filled } => *amount_filled,
                            FillType::Cancelled => {
                                // No settlement needed for cancelled orders (self-trade protection)
                                continue;
                            }
                        };

                        let trade_value = *fill_amount * order_index.price.deref();

                        let (buyer_user_id, seller_user_id) = match order_index.side {
                            OrderSide::Buy => {
                                // Maker is buying (base asset), taker is selling
                                (&maker_order_data.user_id, &args.user_id)
                            }
                            OrderSide::Sell => {
                                // Maker is selling (base asset), taker is buying
                                (&args.user_id, &maker_order_data.user_id)
                            }
                        };

                        let (base_asset, quote_asset) = base_quote;

                        // Transaction 1: Transfer base asset from exchange to buyer
                        let buyer_source_id = format!("user:{}", buyer_user_id.0);
                        let base_currency_str = base_asset.as_str(&self.symbol_vocabulary);

                        sqlx::query!(
                            r#"
                            INSERT INTO t_account_tx_journal
                            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                            VALUES (
                                (SELECT id FROM t_money_accounts WHERE source_type = 'user' AND source_id = $1 AND currency = $2),
                                (SELECT id FROM t_money_accounts WHERE source_type != 'user' AND currency = $2),
                                $2,
                                $3::numeric,
                                $4,
                                $5
                            )
                            "#,
                            buyer_source_id,
                            base_currency_str,
                            fill_amount.deref(),
                            "trade_settlement",
                            uuid::Uuid::new_v4().to_string()
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .map_err(|err| Error::CouldNotPersistToEventSource(err))?;

                        // Transaction 2: Transfer quote asset from exchange to seller
                        let seller_source_id = format!("user:{}", seller_user_id.0);
                        let quote_currency_str = quote_asset.as_str(&self.symbol_vocabulary);

                        sqlx::query!(
                            r#"
                            INSERT INTO t_account_tx_journal
                            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                            VALUES (
                                (SELECT id FROM t_money_accounts WHERE source_type = 'user' AND source_id = $1 AND currency = $2),
                                (SELECT id FROM t_money_accounts WHERE source_type != 'user' AND currency = $2),
                                $2,
                                $3::numeric,
                                $4,
                                $5
                            )
                            "#,
                            seller_source_id,
                            quote_currency_str,
                            trade_value,
                            "trade_settlement",
                            uuid::Uuid::new_v4().to_string()
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .map_err(|err| Error::CouldNotPersistToEventSource(err))?;
                    }

                    match transaction.commit().await {
                        Ok(()) => {
                            // Save values before consuming pending_fill
                            let taker_fill_outcome = pending_fill.taker_fill_outcome;
                            let filled_qty = pending_fill.filled_qty();

                            let match_events = pending_fill
                                .match_events(args.order_uuid, args.order_details.clone());

                            let new_last_trade_price = pending_fill
                                .fills
                                .iter()
                                .filter(|fill| fill.fill_type != FillType::Cancelled)
                                .map(|fill| fill.order_index.price)
                                .last();

                            // Commit fills to orderbook (removes/updates maker orders)
                            let orderbook = pending_fill.commit();

                            // Check if taker order has a resting portion
                            let has_resting_portion = match taker_fill_outcome {
                                None => true,                           // No fills = entire order rests on the book
                                Some(FillType::Partial { .. }) => true, // Partial fill = remainder rests
                                Some(FillType::Complete { .. }) | Some(FillType::Cancelled) => {
                                    false
                                }
                            };

                            vars.last_traded_price = new_last_trade_price;

                            if has_resting_portion {
                                // Calculate remaining quantity to rest on book
                                let total_qty = args
                                    .order_details
                                    .quantity()
                                    .expect("quantity validated earlier");
                                let remaining_qty = *total_qty - filled_qty;

                                // Insert resting order into orderbook and get its OrderIndex
                                let order_index = orderbook.insert(
                                    computed_price,
                                    args.order_uuid,
                                    args.user_id.clone(),
                                    args.order_details.clone(),
                                );

                                // Track the position for this user
                                let open_order = OpenOrder {
                                    order_uuid: args.order_uuid,
                                    order_index,
                                    userref: args.order_details.userref,
                                    cl_ord_id: args.order_details.cl_ord_id.clone(),
                                    base_quote,
                                };

                                self.tracking.send_modify(move |map| {
                                    let profile = map.entry(args.user_id).or_insert(UserProfile {
                                        open_orders: vec![],
                                    });

                                    profile.open_orders.push(open_order);
                                });

                                if let crate::orderbook::TimeInForce::GoodTilDate(expiry_ts) = args.order_details.time_in_force() {
                                    let insert_pos = self.expiry_queue
                                        .binary_search_by_key(&expiry_ts, |(ts, _, _)| *ts)
                                        .unwrap_or_else(|pos| pos);
                                    self.expiry_queue.insert(insert_pos, (expiry_ts, args.order_uuid, Some(order_index)));
                                }
                            }

                            Ok(MsgOut::OrderPlaced)
                        }
                        Err(err) => todo!("{err} {err:?}"),
                    }
                }
                MsgIn::CancelOrderBy {
                    user_id,
                    cancel_order_by,
                } => {
                    // isolate the orders to cancel from the user's open orders
                    let orders_to_cancel: Vec<_> = {
                        self.tracking
                            .borrow()
                            .get(&user_id)
                            .ok_or(Error::NoOpenPositions)?
                            .isolate_orders_for_cancel(&cancel_order_by)
                    };

                    if orders_to_cancel.is_empty() {
                        return Err(Error::OrderNotFound);
                    }

                    let (base, quote) = base_quote;
                    let mut success = vec![];
                    let mut failed = vec![]; // TODO: right now failures arent a thing we either panic or yeet if we cant cancel/refund positions.

                    // Release funds for each cancelled order
                    for open_order in &orders_to_cancel {
                        let order_index = open_order.order_index.clone();
                        let order_data = self.orderbook.get(order_index).expect("always valid");

                        // Release reserved funds back to user
                        // TODO: are Iceberg orders handled?
                        let (currency, reserved_amount) = match order_index.side {
                            OrderSide::Buy => {
                                // Buy orders reserve quote currency (quantity * price)
                                let amount = *order_data.remaining_quantity * *order_data.price;
                                (quote, Money38_18(amount))
                            }
                            OrderSide::Sell => {
                                // Sell orders reserve base currency (quantity)
                                let amount = *order_data.remaining_quantity;
                                (base, Money38_18(amount))
                            }
                        };

                        // Create refund transaction (reverse of reservation)
                        let user_source_id = format!("user:{}", user_id.0);
                        let currency_str = currency.as_str(&self.symbol_vocabulary);

                        sqlx::query!(
                            r#"
                            INSERT INTO t_account_tx_journal
                            (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                            VALUES (
                                (SELECT id FROM t_money_accounts WHERE source_type = 'user' AND source_id = $1 AND currency = $2),
                                (SELECT id FROM t_money_accounts WHERE source_type != 'user' AND currency = $2),
                                $2,
                                $3::numeric,
                                $4,
                                $5
                            )
                            "#,
                            user_source_id,
                            currency_str,
                            reserved_amount.0,
                            "cancel_refund",
                            uuid::Uuid::new_v4().to_string()
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .map_err(|err| Error::CouldNotPersistToEventSource(err))?;

                        success.push(order_data.order_id.clone());
                    }

                    // Commit transaction
                    match transaction.commit().await {
                        Ok(()) => {
                            for open_order in &orders_to_cancel {
                                self.expiry_queue.retain(|(_, uuid, _)| *uuid != open_order.order_uuid);
                            }

                            self.tracking.send_modify(|map| {
                                let profile = map.get_mut(&user_id).expect("user exists");

                                for open_order in &orders_to_cancel {
                                    let order_index = open_order.order_index.clone();
                                    let order_data = self
                                        .orderbook
                                        .remove(order_index.clone())
                                        .expect("always in");
                                }

                                profile.open_orders.extract_if(.., |o| {
                                    orders_to_cancel
                                        .iter()
                                        .find(|t| t.order_uuid == o.order_uuid)
                                        .is_some()
                                });
                            });

                            Ok(MsgOut::OrderCancelled { success, failed })
                        }
                        Err(err) => {
                            tracing::error!(?err, "failed to commit cancellation transaction");
                            Err(Error::CouldNotPersistToEventSource(err))
                        }
                    }
                }
            }
        };

        if let Err(_response) = snd.send(fut.await) {
            tracing::warn!("original message requestor droppped reciever for response")
        }
    }
}

async fn asset_processor_loop(mut mpsc_receiver: mpsc::Receiver<Envelope>, mut proc: ApState) {
    let mut vars = Vars {
        is_suspended: false,
        last_traded_price: None, // TODO: REPLACE THIS WITH AN INDEX PRICE INSTEAD OF NONE
        will_shutdown: false,
    };

    enum Event {
        MsgIn(Option<Envelope>),
        ExpiryReached,
    }

    'rcv: while !vars.will_shutdown {
        let expiry_sleep = match proc.expiry_queue.first() {
            Some((expiry_ts, _, _)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if *expiry_ts <= now {
                    tokio::time::sleep(std::time::Duration::ZERO)
                } else {
                    tokio::time::sleep(std::time::Duration::from_secs(expiry_ts - now))
                }
            }
            None => tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)),
        };

        let event = tokio::select! {
            t = mpsc_receiver.recv() => Event::MsgIn(t),
            _ = expiry_sleep => Event::ExpiryReached,
        };

        match event {
            Event::MsgIn(None) => todo!("no more possible senders, webserver is down?"),
            Event::MsgIn(Some((snd, msg_in))) => proc.switch_msg_in(msg_in, snd, &mut vars).await,
            Event::ExpiryReached => proc.handle_expiry(&mut vars).await,
        }
    }
}

#[derive(Debug)]
pub struct ApInfo {
    pub base_quote: BaseQuote,
    pub base_quote_string: String,
    pub mpsc_sender: mpsc::Sender<Envelope>,
    pub broadcast_sender: broadcast::WeakSender<BroadcastEvent>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

pub async fn launch_ap_procs<'a>(
    pg_pool: sqlx::PgPool,
) -> anyhow::Result<(SymbolVocabulary, Vec<ApInfo>)> {
    let t_trading_asset_pairs: Vec<AssetPairRow> = sqlx::query_as!(
        AssetPairRow,
        "SELECT * FROM t_trading_asset_pairs WHERE status = 'active'",
    )
    .fetch_all(&pg_pool)
    .await
    .context("error fetching active asset pairs")?;

    let symbol_vocabulary = t_trading_asset_pairs
        .iter()
        .map(|r| [r.base_asset.clone(), r.quote_asset.clone()])
        .flatten()
        .unique()
        .collect::<SymbolVocabulary>();

    let mut map: HashMap<String, (ApState, Vars)> = t_trading_asset_pairs
        .into_iter()
        .map(|asset_pair_row| {
            let base_quote_string = format!(
                "{base}/{quote}",
                base = asset_pair_row.base_asset,
                quote = asset_pair_row.quote_asset
            );

            let proc = ApState {
                pg_pool: pg_pool.clone(),
                orderbook: Orderbook::new_empty(),
                tracking: Default::default(),
                asset_pair_row,
                symbol_vocabulary: symbol_vocabulary.clone(),
                expiry_queue: vec![],
            };

            let vars = Vars {
                last_traded_price: None,
                is_suspended: false,
                will_shutdown: false,
            };

            (base_quote_string, (proc, vars))
        })
        .collect();

    let mut t_trading_event_source =
        sqlx::query!(r#"SELECT id, jstr FROM t_trading_event_source"#,).fetch(&pg_pool);

    // stream out rows from the `t_trading_event_source` table, deserialize them into MsgIn and process them
    while let Some(maybe_row) = t_trading_event_source.next().await {
        let record = match maybe_row {
            Ok(record) => record,
            Err(error) => {
                tracing::error!(?error, "error fetching trading event source row");
                return Err(error).context("error fetching trading event source row");
            }
        };

        tracing::trace!(?record.id, "processing trading event source row");
        let Ok((base_quote_st, msg_in)) = serde_json::from_value::<(String, MsgIn)>(record.jstr)
        else {
            tracing::error!(?record.id, "error deserializing trading event source row");
            continue;
        };

        match map.get_mut(&base_quote_st) {
            Some((proc, vars)) => {
                let (snd, _) = oneshot::channel();
                let () = proc.switch_msg_in(msg_in, snd, vars).await;
            }
            None => tracing::trace!("skipping message for deactivated asset pair"),
        };
    }

    let ap_info = map.into_iter().map(| (base_quote_string, (proc, vars)) |{
        let channel_buffer_size = option_env!("AP_CHANNEL_BUFFER_SIZE") // TODO: can we do better?
        .and_then(|st| {
            st.parse()
                .inspect_err(|err| {
                    tracing::error!(?err, ?st, "The environment variable AP_CHANNEL_BUFFER_SIZE could not be parsed to a number (usize)");
                })
                .ok()
        })
        .unwrap_or(1);

        let (mpsc_sender, mpsc_receiver) = tokio::sync::mpsc::channel(channel_buffer_size);
        let (broadcast_sender, _) = broadcast::channel(channel_buffer_size);

        let base_quote = proc
            .asset_pair_row
            .base_quote(&symbol_vocabulary)
            .expect("base or quote was not in symbol vocabulary");


        let span = tracing::info_span!("asset_processor_loop", asset_pair = proc.asset_pair_row.id);
        let join_handle = tokio::task::spawn(asset_processor_loop(mpsc_receiver, proc).instrument(
            span,
        ));

        ApInfo {
            base_quote,
            base_quote_string,
            mpsc_sender,
            broadcast_sender: broadcast_sender.downgrade(),
            join_handle: Some(join_handle),
        }
    }).collect();

    Ok((symbol_vocabulary, ap_info))
}

#[cfg(test)]
mod test;
