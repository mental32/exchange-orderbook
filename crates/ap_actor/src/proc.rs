use crate::VirtualUserId;
use crate::market_snapshot::DepthLevel;
use crate::proc_router::CancelOrderBy;
use crate::proc_router::PlaceOrderArgs;
use crate::reserve_money::ReserveByAssetError;
use crate::user_profile::OpenOrder;
use crate::user_profile::UserProfile;
use futures::StreamExt as _;
use itertools::Itertools as _;
use matching_engine::asset_code::AssetCode;
use matching_engine::asset_code::SymbolVocabulary;
use matching_engine::asset_pair::AssetPairRow;
use matching_engine::asset_pair::BaseQuote;
use matching_engine::decimal::Decimal;
use matching_engine::decimal::NonZeroDecimal;
use matching_engine::money38_18::Money38_18;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderIndex;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::orderbook::Orderbook;
use matching_engine::orderbook::TimeInForce;
use matching_engine::pending_fill::Fill;
use matching_engine::pending_fill::FillType;
use matching_engine::pending_fill::TifViolation;
use matching_engine::price::Price;
use matching_engine::try_fill_order::TryFillOrdersError;
use sqlx::postgres::PgListener;
use sqlx::postgres::PgNotification;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::ops::Deref;
use std::ops::DerefMut;
use std::time::SystemTime;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::Instrument as _;

#[derive(Debug, Clone)]
enum TriggerOrderType {
    StopLoss,
    TakeProfit,
}

/// Entry for a trigger order (stop-loss or take-profit) awaiting trigger
#[derive(Debug)]
struct TriggerOrderEntry {
    order_type: TriggerOrderType,
    order_uuid: OrderUuid,
    user_id: VirtualUserId,
    args: PlaceOrderArgs,
    trigger_price: NonZeroDecimal,
    limit_price: Option<NonZeroDecimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderDescriptor {
    pub order_type: OrderType,
    pub side: OrderSide,
    pub filled_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub limit_price: Option<NonZeroDecimal>,
    pub trigger_price: Option<NonZeroDecimal>,
    pub display_quantity: Option<NonZeroDecimal>,
    pub userref: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderDescription {
    pub order: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<uuid::Uuid>,
    pub descr: OrderDescription,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickerSnapshot {
    pub last_trade_price: Option<Decimal>,
    pub last_trade_volume: Option<Decimal>,
    pub best_bid: Option<DepthLevel>,
    pub best_ask: Option<DepthLevel>,
    pub opening_price_today: Option<Decimal>,
    pub high_today: Option<Decimal>,
    pub low_today: Option<Decimal>,
    pub vwap_today: Option<Decimal>,
    pub high_24h: Option<Decimal>,
    pub low_24h: Option<Decimal>,
    pub vwap_24h: Option<Decimal>,
    pub volume_today: Decimal,
    pub volume_24h: Decimal,
    pub trades_today: u64,
    pub trades_24h: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AmendOrderArgs {
    pub user_id: VirtualUserId,
    pub order_uuid: OrderUuid,
    pub new_order_qty: Option<Decimal>,
    pub new_display_qty: Option<Decimal>,
    pub new_limit_price: Option<NonZeroDecimal>,
    pub new_trigger_price: Option<NonZeroDecimal>,
    pub post_only: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CancelOrderByArgs {
    pub user_id: VirtualUserId,
    pub cancel_order_by: CancelOrderBy,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DescribeOrderArgs {
    pub user_id: VirtualUserId,
    pub order_uuid: OrderUuid,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MsgIn {
    PlaceOrder(PlaceOrderArgs),
    CancelOrderBy(CancelOrderByArgs),
    AmendOrder(AmendOrderArgs),
    DescribeOrder(DescribeOrderArgs),
    TickerSnapshot,
    SetStatus(ProcStatus),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MsgOut {
    OrderPlaced,
    OrderValidated,
    OrderCancelled {
        success: Vec<OrderUuid>,
        failed: Vec<OrderUuid>,
    },
    OrderAmended {
        order_uuid: OrderUuid,
    },
    OrderSnapshot {
        descriptor: OrderDescriptor,
    },
    TickerSnapshot {
        snapshot: TickerSnapshot,
    },
    StatusChanged(ProcStatus),
    ShutdownAcknowledged,
}

pub type Response = Result<MsgOut, MsgError>;

pub type Envelope = (oneshot::Sender<Response>, MsgIn);

#[derive(Debug, thiserror::Error)]
pub enum MsgError {
    #[error("message serialization error")]
    UnserializableInput {
        base_quote: (AssetCode, AssetCode),
        message: MsgIn,
        #[source]
        serde_json_error: serde_json::Error,
    },
    #[error("message-in could not be persisted to event table: {0}")]
    CouldNotPersistToEventSource(sqlx::Error),
    #[error("money account not found for user {user_id} in asset {asset_code:?}")]
    UserAccountNotFound {
        user_id: VirtualUserId,
        asset_code: AssetCode,
    },
    #[error("database error while updating balances: {0}")]
    BalanceDatabase(#[source] sqlx::Error),
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
    #[error("cannot reduce order quantity below filled quantity")]
    CannotReduceBelowFilled,
    #[error("post_only amendment would cross the spread")]
    PostOnlyWouldCross,
    #[error("display quantity must be >= 1/15 of remaining quantity")]
    InvalidDisplayQuantity,
}

impl PartialEq for MsgError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::UnserializableInput {
                    base_quote: l_base_quote,
                    message: l_message,
                    serde_json_error: _,
                },
                Self::UnserializableInput {
                    base_quote: r_base_quote,
                    message: r_message,
                    serde_json_error: _,
                },
            ) => {
                l_base_quote == r_base_quote && l_message == r_message
                // && l_serde_json_error == r_serde_json_error
            }
            (Self::CouldNotPersistToEventSource(l0), Self::CouldNotPersistToEventSource(r0)) => l0
                .as_database_error()
                .zip(r0.as_database_error())
                .and_then(|(l, r)| l.code().zip(r.code()))
                .map(|(l, r)| l == r)
                .unwrap_or(false),
            (
                Self::UserAccountNotFound {
                    user_id: l_user_id,
                    asset_code: l_asset_code,
                },
                Self::UserAccountNotFound {
                    user_id: r_user_id,
                    asset_code: r_asset_code,
                },
            ) => l_user_id == r_user_id && l_asset_code == r_asset_code,
            (Self::BalanceDatabase(l0), Self::BalanceDatabase(r0)) => l0
                .as_database_error()
                .zip(r0.as_database_error())
                .and_then(|(l, r)| l.code().zip(r.code()))
                .map(|(l, r)| l == r)
                .unwrap_or(false),
            (Self::TifViolation(l0), Self::TifViolation(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettleTransferError {
    #[error("insufficient funds for user {user_id} in {asset_code:?} account")]
    InsufficientFunds {
        user_id: VirtualUserId,
        asset_code: AssetCode,
    },
    #[error("account not found for user {user_id} in {asset_code:?}")]
    AccountNotFound {
        user_id: VirtualUserId,
        asset_code: AssetCode,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BroadcastEvent {
    Reserve {
        user_id: VirtualUserId,
        amount: Decimal,
        asset_code: AssetCode,
    },
    Settlement {
        buyer_id: VirtualUserId,
        seller_id: VirtualUserId,
        base_amount: Decimal,
        quote_amount: Decimal,
        base_asset: AssetCode,
        quote_asset: AssetCode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProcStatus {
    /// All order types may be submitted and trades can occur.
    Online,
    /// The exchange is offline. No new orders or cancellations may be submitted.
    Maintenance,
    /// Resting (open) orders can be cancelled but no new orders may be submitted. No trades will occur.
    CancelOnly,
    /// Only post-only limit orders can be submitted. Existing orders may still be cancelled. No trades will occur.
    PostOnly,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum BalanceKey {
    Base(VirtualUserId),
    Quote(VirtualUserId),
}

struct Balances(HashMap<BalanceKey, Money38_18>);

impl Deref for Balances {
    type Target = HashMap<BalanceKey, Money38_18>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Balances {
    fn atomic_four_way_double_entry_transfer(
        &mut self,
        debit_base_user: VirtualUserId,
        base_amount: Money38_18,
        debit_quote_user: VirtualUserId,
        quote_amount: Money38_18,
        (base, quote): BaseQuote,
    ) -> Result<BroadcastEvent, SettleTransferError> {
        let debit_base_key = BalanceKey::Base(debit_base_user);
        let credit_base_key = BalanceKey::Base(debit_quote_user);
        let debit_quote_key = BalanceKey::Quote(debit_quote_user);
        let credit_quote_key = BalanceKey::Quote(debit_base_user);

        let debit_base_balance =
            self.0
                .get(&debit_base_key)
                .ok_or(SettleTransferError::AccountNotFound {
                    user_id: debit_base_user,
                    asset_code: base.clone(),
                })?;

        if debit_base_balance.0 < base_amount.0 {
            return Err(SettleTransferError::InsufficientFunds {
                user_id: debit_base_user,
                asset_code: base.clone(),
            });
        }

        let debit_quote_balance =
            self.0
                .get(&debit_quote_key)
                .ok_or(SettleTransferError::AccountNotFound {
                    user_id: debit_quote_user,
                    asset_code: quote.clone(),
                })?;

        if debit_quote_balance.0 < quote_amount.0 {
            return Err(SettleTransferError::InsufficientFunds {
                user_id: debit_quote_user,
                asset_code: quote.clone(),
            });
        }

        self.0
            .get_mut(&debit_base_key)
            .expect("already validated")
            .0 -= base_amount.0;

        self.0
            .entry(credit_base_key)
            .or_insert(Money38_18(Decimal::ZERO))
            .0 += base_amount.0;

        self.0
            .get_mut(&debit_quote_key)
            .expect("already validated")
            .0 -= quote_amount.0;

        self.0
            .entry(credit_quote_key)
            .or_insert(Money38_18(Decimal::ZERO))
            .0 += quote_amount.0;

        Ok(BroadcastEvent::Settlement {
            buyer_id: debit_quote_user,
            seller_id: debit_base_user,
            base_amount: base_amount.0,
            quote_amount: quote_amount.0,
            base_asset: base,
            quote_asset: quote,
        })
    }

    fn reserve_money_by_asset(
        &mut self,
        balance_key: BalanceKey,
        Money38_18(delta): Money38_18,
        (base, quote): BaseQuote,
    ) -> Result<BroadcastEvent, ReserveByAssetError> {
        let acc = self
            .0
            .get_mut(&balance_key)
            .ok_or(ReserveByAssetError::AccountNotFound)?;

        if acc.0 - delta < Decimal::ZERO {
            return Err(ReserveByAssetError::InsufficientFunds);
        }

        acc.0 = acc.0 - delta;

        Ok(BroadcastEvent::Reserve {
            user_id: match balance_key {
                BalanceKey::Base(uid) | BalanceKey::Quote(uid) => uid,
            },
            amount: delta,
            asset_code: match balance_key {
                BalanceKey::Base(_) => base,
                BalanceKey::Quote(_) => quote,
            },
        })
    }
}

#[derive(Debug, Clone)]
struct TickerTrade {
    timestamp: SystemTime,
    price: Decimal,
    volume: Decimal,
}

struct Proc {
    pg_pool: sqlx::PgPool,
    orderbook: Orderbook<VirtualUserId>,
    profiles: HashMap<VirtualUserId, UserProfile>,
    balances: Balances,
    asset_pair_row: AssetPairRow,
    symbol_vocabulary: SymbolVocabulary,
    expiry_queue: Vec<(u64, OrderUuid, Option<OrderIndex>)>,
    trigger_orders: Vec<TriggerOrderEntry>,
    last_traded_price: Option<NonZeroDecimal>,
    last_trade_volume: Option<Decimal>,
    ticker_trades: VecDeque<TickerTrade>,
    status: ProcStatus,
    will_shutdown: bool,
    // start_time: (Instant, SystemTime),
}

type SwitchMsgInOutput = Result<MsgOut, MsgError>;

impl Proc {
    fn prune_ticker_trades(&mut self, now: SystemTime) {
        let window_start = now
            .checked_sub(std::time::Duration::from_secs(24 * 60 * 60))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        while let Some(front) = self.ticker_trades.front() {
            if front.timestamp < window_start {
                self.ticker_trades.pop_front();
            } else {
                break;
            }
        }

        if let Some(last) = self.ticker_trades.back() {
            self.last_trade_volume = Some(last.volume);
        } else {
            self.last_trade_volume = None;
        }
    }

    fn record_trade(&mut self, price: Decimal, volume: Decimal, now: SystemTime) {
        if volume <= Decimal::ZERO {
            return;
        }

        self.ticker_trades.push_back(TickerTrade {
            timestamp: now,
            price,
            volume,
        });
        self.last_trade_volume = Some(volume);
        self.prune_ticker_trades(now);
    }

    fn ticker_snapshot(&mut self, now: SystemTime) -> TickerSnapshot {
        self.prune_ticker_trades(now);

        let window_start = now
            .checked_sub(std::time::Duration::from_secs(24 * 60 * 60))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let now_secs = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let day_start_secs = now_secs - (now_secs % (24 * 60 * 60));
        let day_start = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(day_start_secs);

        let mut volume_24h = Decimal::ZERO;
        let mut trades_24h = 0u64;
        let mut vwap_num_24h = Decimal::ZERO;
        let mut high_24h: Option<Decimal> = None;
        let mut low_24h: Option<Decimal> = None;

        let mut volume_today = Decimal::ZERO;
        let mut trades_today = 0u64;
        let mut vwap_num_today = Decimal::ZERO;
        let mut high_today: Option<Decimal> = None;
        let mut low_today: Option<Decimal> = None;
        let mut opening_price_today: Option<(SystemTime, Decimal)> = None;

        for trade in self.ticker_trades.iter() {
            if trade.timestamp >= window_start {
                volume_24h += trade.volume;
                trades_24h += 1;
                vwap_num_24h += trade.price * trade.volume;
                high_24h = Some(high_24h.map_or(trade.price, |v| v.max(trade.price)));
                low_24h = Some(low_24h.map_or(trade.price, |v| v.min(trade.price)));
            }

            if trade.timestamp >= day_start {
                volume_today += trade.volume;
                trades_today += 1;
                vwap_num_today += trade.price * trade.volume;
                high_today = Some(high_today.map_or(trade.price, |v| v.max(trade.price)));
                low_today = Some(low_today.map_or(trade.price, |v| v.min(trade.price)));

                opening_price_today = match opening_price_today {
                    Some((ts, price)) if ts <= trade.timestamp => Some((ts, price)),
                    _ => Some((trade.timestamp, trade.price)),
                };
            }
        }

        let best_bid = self.orderbook.bids().next().map(|(_, order)| DepthLevel {
            price: *order.price.deref(),
            volume: *order.remaining_quantity.deref(),
            timestamp: now_secs,
        });

        let best_ask = self.orderbook.asks().next().map(|(_, order)| DepthLevel {
            price: *order.price.deref(),
            volume: *order.remaining_quantity.deref(),
            timestamp: now_secs,
        });

        TickerSnapshot {
            last_trade_price: self.last_traded_price.map(|p| *p.deref()),
            last_trade_volume: self.last_trade_volume,
            best_bid,
            best_ask,
            opening_price_today: opening_price_today.map(|(_, price)| price),
            high_today,
            low_today,
            vwap_today: (volume_today > Decimal::ZERO).then_some(vwap_num_today / volume_today),
            high_24h,
            low_24h,
            vwap_24h: (volume_24h > Decimal::ZERO).then_some(vwap_num_24h / volume_24h),
            volume_today,
            volume_24h,
            trades_today,
            trades_24h,
        }
    }

    fn handle_expiry(&mut self, events: &mut Vec<BroadcastEvent>) {
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

        let base_quote = self
            .asset_pair_row
            .base_quote(&self.symbol_vocabulary)
            .expect("symbols are always resolvable");
        let (base, quote) = base_quote;

        for (_, order_uuid, order_index_opt) in expired {
            if let Some(trigger_idx) = self
                .trigger_orders
                .iter()
                .position(|entry| entry.order_uuid == order_uuid)
            {
                let entry = self.trigger_orders.remove(trigger_idx);

                let (currency, refund_amount) = {
                    let quantity = *entry.args.order_details.quantity.unwrap().deref();
                    match entry.args.order_details.side {
                        OrderSide::Buy => {
                            // For buy orders, refund quantity * trigger_price in quote currency
                            let amount = quantity * *entry.trigger_price.deref();
                            (quote.clone(), Money38_18(amount))
                        }
                        OrderSide::Sell => {
                            // For sell orders, refund quantity in base currency
                            (base.clone(), Money38_18(quantity))
                        }
                    }
                };

                let balance_key = if currency == base {
                    BalanceKey::Base(entry.user_id)
                } else {
                    BalanceKey::Quote(entry.user_id)
                };

                debug_assert!(
                    refund_amount.0 > Decimal::ZERO,
                    "refund amount must be positive"
                );
                match self.balances.reserve_money_by_asset(
                    balance_key,
                    Money38_18(-refund_amount.0),
                    self.asset_pair_row
                        .base_quote(&self.symbol_vocabulary)
                        .unwrap(),
                ) {
                    Ok(event) => events.push(event),
                    Err(err) => {
                        tracing::error!(?err, "trigger expiry refund failed");
                        continue;
                    }
                }

                continue;
            }

            if let Some(order_index) = order_index_opt {
                let order_data = self
                    .orderbook
                    .get(order_index)
                    .expect("order exists in book");

                let (currency, reserved_amount) = match order_index.side {
                    OrderSide::Buy => {
                        let amount = *order_data.remaining_quantity * *order_data.price;
                        (quote.clone(), Money38_18(amount))
                    }
                    OrderSide::Sell => {
                        let amount = *order_data.remaining_quantity;
                        (base.clone(), Money38_18(amount))
                    }
                };

                let balance_key = if currency == base {
                    BalanceKey::Base(order_data.user_id)
                } else {
                    BalanceKey::Quote(order_data.user_id)
                };

                debug_assert!(
                    reserved_amount.0 > Decimal::ZERO,
                    "reserved amount must be positive"
                );
                match self.balances.reserve_money_by_asset(
                    balance_key,
                    Money38_18(-reserved_amount.0),
                    self.asset_pair_row
                        .base_quote(&self.symbol_vocabulary)
                        .unwrap(),
                ) {
                    Ok(event) => events.push(event),
                    Err(err) => {
                        tracing::error!(?err, "expiry refund failed");
                        continue;
                    }
                }

                self.orderbook
                    .remove(order_index)
                    .expect("order exists in book");
            }

            for (_, profile) in self.profiles.iter_mut() {
                profile.open_orders.retain(|o| o.order_uuid != order_uuid);
            }
        }
    }

    fn switch_msg_in(&mut self, msg_in: MsgIn) -> (SwitchMsgInOutput, Vec<BroadcastEvent>) {
        use MsgIn as M;
        use ProcStatus as S;

        let mut events = vec![];

        let switch_msg_in_output = match (msg_in, &mut self.status) {
            (M::SetStatus(status), place) => {
                *place = status.clone();
                Ok(MsgOut::StatusChanged(status))
            }
            (M::Shutdown, _) => {
                self.will_shutdown = true;
                Ok(MsgOut::ShutdownAcknowledged)
            }
            (M::TickerSnapshot, _) => Ok(MsgOut::TickerSnapshot {
                snapshot: self.ticker_snapshot(SystemTime::now()),
            }),
            (M::DescribeOrder(args), _) => self.describe_order(args, &mut events),
            (_, S::Maintenance) => Err(MsgError::ProcessorIsSuspended),
            (M::PlaceOrder(args), S::Online | S::PostOnly) => self.place_order(args, &mut events),
            (M::CancelOrderBy(args), S::Online | S::CancelOnly) => {
                self.cancel_order_by(args, &mut events)
            }
            (M::AmendOrder(args), S::Online) => self.amend_order(args, &mut events),
            _ => unreachable!(),
        };

        (switch_msg_in_output, events)
    }

    fn describe_order(
        &mut self,
        DescribeOrderArgs {
            user_id,
            order_uuid,
        }: DescribeOrderArgs,
        _events: &mut Vec<BroadcastEvent>,
    ) -> SwitchMsgInOutput {
        let descriptor = if let Some(entry_idx) = self
            .trigger_orders
            .iter()
            .position(|entry| entry.order_uuid == order_uuid && entry.user_id == user_id)
        {
            let entry = &self.trigger_orders[entry_idx];
            let quantity = entry
                .args
                .order_details
                .quantity
                .map(|q| *q.deref())
                .unwrap_or(Decimal::ZERO);

            OrderDescriptor {
                order_type: entry.args.order_details.order_type,
                side: entry.args.order_details.side,
                filled_quantity: Decimal::ZERO,
                remaining_quantity: quantity,
                limit_price: entry.limit_price,
                trigger_price: Some(entry.trigger_price),
                display_quantity: entry.args.order_details.display_quantity,
                userref: entry.args.order_details.userref,
            }
        } else {
            let open_order = match self
                .profiles
                .get(&user_id)
                .and_then(|profile| {
                    profile
                        .open_orders
                        .iter()
                        .find(|o| o.order_uuid == order_uuid)
                        .cloned()
                })
                .ok_or(MsgError::OrderNotFound)
            {
                Ok(v) => v,
                Err(e) => return Err(e),
            };

            let order_index = open_order.order_index;
            let order_data = match self
                .orderbook
                .get(order_index)
                .ok_or(MsgError::OrderNotFound)
            {
                Ok(v) => v,
                Err(e) => return Err(e),
            };

            let order_type = if order_data.display_quantity.is_some() {
                OrderType::Iceberg
            } else {
                OrderType::Limit
            };

            OrderDescriptor {
                order_type,
                side: order_index.side,
                filled_quantity: order_data.filled_quantity,
                remaining_quantity: *order_data.remaining_quantity,
                limit_price: Some(order_data.price),
                trigger_price: None,
                display_quantity: order_data.display_quantity,
                userref: order_data.userref,
            }
        };

        Ok(MsgOut::OrderSnapshot { descriptor })
    }

    fn amend_order(
        &mut self,
        args: AmendOrderArgs,
        events: &mut Vec<BroadcastEvent>,
    ) -> SwitchMsgInOutput {
        let user_id = &args.user_id;
        let order_uuid = args.order_uuid;

        let (base, quote) = self
            .asset_pair_row
            .base_quote(&self.symbol_vocabulary)
            .expect("symbols are always resolvable")
            .clone();

        // Check if it's a trigger order (stop-loss or take-profit)
        if let Some(trigger_idx) = self
            .trigger_orders
            .iter()
            .position(|entry| entry.order_uuid == order_uuid && entry.user_id == *user_id)
        {
            let entry = &mut self.trigger_orders[trigger_idx];

            // Get current order quantity
            let current_qty = *entry.args.order_details.quantity.unwrap().deref();

            // Validate new_order_qty if provided
            if let Some(new_qty) = args.new_order_qty {
                if new_qty <= Decimal::ZERO {
                    return Err(MsgError::ZeroQuantity);
                }
                // Trigger orders have no fills yet, but we still validate against zero
            }

            // Calculate fund delta for quantity changes
            let new_qty = args.new_order_qty.unwrap_or(current_qty);
            let qty_delta = new_qty - current_qty;

            // Calculate fund delta for trigger price changes
            let new_trigger_price = args.new_trigger_price.unwrap_or(entry.trigger_price);

            // Determine if we need additional funds or should refund
            let (currency, fund_delta) = match entry.args.order_details.side {
                OrderSide::Buy => {
                    // Buy: reserve quote currency (qty * trigger_price)
                    let old_reserve = current_qty * *entry.trigger_price.deref();
                    let new_reserve = new_qty * *new_trigger_price.deref();
                    (quote, Money38_18(new_reserve - old_reserve))
                }
                OrderSide::Sell => {
                    // Sell: reserve base currency (qty)
                    (base.clone(), Money38_18(qty_delta))
                }
            };

            // Apply fund changes if needed
            if fund_delta.0 != Decimal::ZERO {
                let balance_key = if currency == base {
                    BalanceKey::Base(*user_id)
                } else {
                    BalanceKey::Quote(*user_id)
                };

                // Positive delta = reserve, negative delta = refund
                match self.balances.reserve_money_by_asset(
                    balance_key,
                    fund_delta,
                    self.asset_pair_row
                        .base_quote(&self.symbol_vocabulary)
                        .unwrap(),
                ) {
                    Ok(event) => events.push(event),
                    Err(err) => {
                        return Err(match err {
                            ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                            ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                                user_id: *user_id,
                                asset_code: currency.clone(),
                            },
                            ReserveByAssetError::Database(error) => {
                                MsgError::BalanceDatabase(error)
                            }
                        });
                    }
                }
            }

            // Update the trigger order entry
            let entry = &mut self.trigger_orders[trigger_idx];
            if let Some(new_qty) = args.new_order_qty {
                entry
                    .args
                    .order_details
                    .quantity
                    .replace(NonZeroDecimal::new(new_qty).unwrap());
            }
            if let Some(new_trigger) = args.new_trigger_price {
                entry.trigger_price = new_trigger;
            }
            if let Some(new_limit) = args.new_limit_price {
                entry.limit_price = Some(new_limit);
            }

            return Ok(MsgOut::OrderAmended { order_uuid });
        }

        let (order_index, base_quote) = {
            let profile = match self.profiles.get(user_id).ok_or(MsgError::OrderNotFound) {
                Ok(profile) => profile,
                Err(err) => return Err(err),
            };
            let open = match profile
                .open_orders
                .iter()
                .find(|o| o.order_uuid == order_uuid)
                .ok_or(MsgError::OrderNotFound)
            {
                Ok(open) => open,
                Err(err) => return Err(err),
            };
            (open.order_index, open.base_quote.clone())
        };

        let order_data = match self
            .orderbook
            .get(order_index)
            .ok_or(MsgError::OrderNotFound)
        {
            Ok(data) => data,
            Err(err) => return Err(err),
        };

        let current_price_nz = order_data.price;
        let current_price_dec = *current_price_nz.deref();
        let current_remaining_dec = *order_data.remaining_quantity;
        let filled_qty = order_data.filled_quantity;
        let current_display_dec = order_data.display_quantity.map(|dq| *dq.deref());
        let side = order_index.side;

        let new_price_nz = args.new_limit_price.unwrap_or(current_price_nz);
        let new_price_dec = *new_price_nz.deref();
        let price_changed = new_price_nz != current_price_nz;

        let original_total_qty = filled_qty + current_remaining_dec;
        let new_total_qty = args.new_order_qty.unwrap_or(original_total_qty);
        if new_total_qty <= Decimal::ZERO {
            return Err(MsgError::ZeroQuantity);
        }

        let new_remaining_dec = if new_total_qty <= filled_qty {
            Decimal::ZERO
        } else {
            new_total_qty - filled_qty
        };

        let mut target_display_dec = match args.new_display_qty {
            Some(d) => Some(d),
            None => current_display_dec,
        };

        if let Some(display) = target_display_dec {
            if display <= Decimal::ZERO {
                return Err(MsgError::InvalidDisplayQuantity);
            }
            if new_remaining_dec != Decimal::ZERO && display > new_remaining_dec {
                return Err(MsgError::InvalidDisplayQuantity);
            }
        } else if let Some(display) = current_display_dec {
            if new_remaining_dec != Decimal::ZERO && display > new_remaining_dec {
                target_display_dec = Some(new_remaining_dec);
            }
        }

        let current_total_cost = match side {
            OrderSide::Buy => current_remaining_dec * current_price_dec,
            OrderSide::Sell => current_remaining_dec,
        };

        let new_total_cost = match side {
            OrderSide::Buy => new_remaining_dec * new_price_dec,
            OrderSide::Sell => new_remaining_dec,
        };

        let delta_amount = new_total_cost - current_total_cost;

        if delta_amount != Decimal::ZERO {
            let balance_key = match side {
                OrderSide::Buy => BalanceKey::Quote(*user_id),
                OrderSide::Sell => BalanceKey::Base(*user_id),
            };

            match self.balances.reserve_money_by_asset(
                balance_key,
                Money38_18(delta_amount),
                base_quote.clone(),
            ) {
                Ok(event) => events.push(event),
                Err(err) => {
                    return Err(match err {
                        ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                        ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                            user_id: *user_id,
                            asset_code: match balance_key {
                                BalanceKey::Base(_) => base_quote.0.clone(),
                                BalanceKey::Quote(_) => base_quote.1.clone(),
                            },
                        },
                        ReserveByAssetError::Database(error) => MsgError::BalanceDatabase(error),
                    });
                }
            }
        }

        if new_remaining_dec == Decimal::ZERO {
            self.orderbook
                .remove(order_index)
                .ok_or(MsgError::OrderNotFound)
                .unwrap();

            if let Some(profile) = self.profiles.get_mut(user_id) {
                profile.open_orders.retain(|o| o.order_uuid != order_uuid);
            }

            self.expiry_queue.retain(|(_, uuid, _)| *uuid != order_uuid);

            return Ok(MsgOut::OrderAmended { order_uuid });
        }

        let new_remaining_nz = NonZeroDecimal::new(new_remaining_dec)
            .map_err(|()| MsgError::ZeroQuantity)
            .unwrap();

        let new_display_nz = match target_display_dec {
            Some(display) => Some(
                NonZeroDecimal::new(display)
                    .map_err(|()| MsgError::InvalidDisplayQuantity)
                    .unwrap(),
            ),
            None => None,
        };

        if price_changed {
            let removed = self
                .orderbook
                .remove(order_index)
                .ok_or(MsgError::OrderNotFound)
                .unwrap();

            let new_index = self.orderbook.insert_amended(
                side,
                new_price_nz,
                order_uuid,
                *user_id,
                new_remaining_nz,
                removed.filled_quantity,
                new_display_nz,
                removed.userref,
            );

            if let Some(profile) = self.profiles.get_mut(user_id) {
                if let Some(open) = profile
                    .open_orders
                    .iter_mut()
                    .find(|o| o.order_uuid == order_uuid)
                {
                    open.order_index = new_index;
                }
            }

            for entry in self.expiry_queue.iter_mut() {
                if entry.1 == order_uuid {
                    entry.2 = Some(new_index);
                }
            }
        } else {
            let order_entry = self
                .orderbook
                .get_mut(order_index)
                .ok_or(MsgError::OrderNotFound)
                .unwrap();

            if new_remaining_dec != current_remaining_dec {
                order_entry.remaining_quantity = new_remaining_nz;
            }

            match (new_display_nz, args.new_display_qty.is_some()) {
                (Some(display), _) => {
                    order_entry.display_quantity = Some(display);
                }
                (None, true) => {
                    order_entry.display_quantity = None;
                }
                _ => {}
            }

            if price_changed {
                order_entry.price = new_price_nz;
            }
        }

        Ok(MsgOut::OrderAmended { order_uuid })
    }

    fn place_order(
        &mut self,
        mut args: PlaceOrderArgs,
        events: &mut Vec<BroadcastEvent>,
    ) -> SwitchMsgInOutput {
        let now_time = SystemTime::now();
        let now = now_time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(expiry_ts) = args.order_details.expiry_time.to_absolute_timestamp(now) {
            args.order_details.time_in_force = TimeInForce::GoodTilDate(expiry_ts);
        }

        let computed_price: NonZeroDecimal = if args.order_details.order_type == OrderType::Market {
            // Sentinel price for market orders (ignored by matching engine)
            NonZeroDecimal::new(Decimal::ONE).unwrap()
        } else if let Some(last_traded_price) = self.last_traded_price {
            match args.order_details.price.compute_to_decimal(
                last_traded_price,
                args.order_details.side,
                args.order_details.order_type,
            ) {
                Ok(v) => v,
                Err(matching_engine::price::InvalidPrice) => return Err(MsgError::InvalidPrice),
            }
        } else {
            if args.order_details.price.prefix.is_none() && args.order_details.price.is_percentage {
                return Err(MsgError::InvalidPrice);
            } else if args.order_details.price.is_relative() {
                return Err(MsgError::NoReferencePrice);
            } else {
                match NonZeroDecimal::new(args.order_details.price.amount) {
                    Ok(v) => v,
                    Err(()) => return Err(MsgError::InvalidPrice),
                }
            }
        };

        if args.order_details.order_type == OrderType::StopLoss
            || args.order_details.order_type == OrderType::StopLossLimit
        {
            let base_quote = args.base_quote.clone();
            let (base, quote) = base_quote.clone();

            let (currency, reserve_amount) = {
                let quantity = *args.order_details.quantity.unwrap().deref();
                match args.order_details.side {
                    OrderSide::Buy => {
                        // For buy orders, reserve quantity * trigger_price in quote currency
                        let amount = quantity * *computed_price.deref();
                        (quote.clone(), Money38_18(amount))
                    }
                    OrderSide::Sell => {
                        // For sell orders, reserve quantity in base currency
                        (base.clone(), Money38_18(quantity))
                    }
                }
            };

            let balance_key = if currency == base {
                BalanceKey::Base(args.user_id)
            } else {
                BalanceKey::Quote(args.user_id)
            };

            debug_assert!(
                reserve_amount.0 > Decimal::ZERO,
                "reserve amount must be positive"
            );
            match self.balances.reserve_money_by_asset(
                balance_key,
                reserve_amount,
                self.asset_pair_row
                    .base_quote(&self.symbol_vocabulary)
                    .unwrap(),
            ) {
                Ok(event) => events.push(event),
                Err(err) => {
                    return Err(match err {
                        ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                        ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                            user_id: args.user_id,
                            asset_code: currency.clone(),
                        },
                        ReserveByAssetError::Database(error) => MsgError::BalanceDatabase(error),
                    });
                }
            }

            let limit_price = if args.order_details.order_type == OrderType::StopLossLimit {
                let secondary_price = match args.order_details.secondary_price {
                    Some(v) => v,
                    None => return Err(MsgError::InvalidPrice),
                };

                let limit_decimal = if let Some(last_traded_price) = self.last_traded_price {
                    match secondary_price.compute_to_decimal(
                        last_traded_price,
                        args.order_details.side,
                        args.order_details.order_type,
                    ) {
                        Ok(v) => v,
                        Err(_) => return Err(MsgError::InvalidPrice),
                    }
                } else {
                    if secondary_price.is_relative() {
                        return Err(MsgError::NoReferencePrice);
                    } else {
                        match NonZeroDecimal::new(secondary_price.amount) {
                            Ok(v) => v,
                            Err(()) => return Err(MsgError::InvalidPrice),
                        }
                    }
                };
                Some(limit_decimal)
            } else {
                None
            };

            self.trigger_orders.push(TriggerOrderEntry {
                order_type: TriggerOrderType::StopLoss,
                order_uuid: args.order_uuid,
                user_id: args.user_id.clone(),
                args: args.clone(),
                trigger_price: computed_price,
                limit_price,
            });

            if let TimeInForce::GoodTilDate(expiry_ts) = args.order_details.time_in_force {
                let insert_pos = self
                    .expiry_queue
                    .binary_search_by_key(&expiry_ts, |(ts, _, _)| *ts)
                    .unwrap_or_else(|pos| pos);
                self.expiry_queue
                    .insert(insert_pos, (expiry_ts, args.order_uuid, None));
            }

            return Ok(MsgOut::OrderPlaced);
        }

        if args.order_details.order_type == OrderType::TakeProfit
            || args.order_details.order_type == OrderType::TakeProfitLimit
        {
            let base_quote = args.base_quote.clone();
            let (base, quote) = base_quote.clone();

            let (currency, reserve_amount) = {
                let quantity = *args.order_details.quantity.unwrap().deref();
                match args.order_details.side {
                    OrderSide::Buy => {
                        // For buy orders, reserve quantity * trigger_price in quote currency
                        let amount = quantity * *computed_price.deref();
                        (quote.clone(), Money38_18(amount))
                    }
                    OrderSide::Sell => {
                        // For sell orders, reserve quantity in base currency
                        (base.clone(), Money38_18(quantity))
                    }
                }
            };

            let balance_key = if currency == base {
                BalanceKey::Base(args.user_id)
            } else {
                BalanceKey::Quote(args.user_id)
            };

            debug_assert!(
                reserve_amount.0 > Decimal::ZERO,
                "reserve amount must be positive"
            );
            match self.balances.reserve_money_by_asset(
                balance_key,
                reserve_amount,
                self.asset_pair_row
                    .base_quote(&self.symbol_vocabulary)
                    .unwrap(),
            ) {
                Ok(event) => events.push(event),
                Err(err) => {
                    return Err(match err {
                        ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                        ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                            user_id: args.user_id,
                            asset_code: currency.clone(),
                        },
                        ReserveByAssetError::Database(error) => MsgError::BalanceDatabase(error),
                    });
                }
            }

            let limit_price = if args.order_details.order_type == OrderType::TakeProfitLimit {
                let secondary_price = match args.order_details.secondary_price {
                    Some(v) => v,
                    None => return Err(MsgError::InvalidPrice),
                };

                let limit_decimal = if let Some(last_traded_price) = self.last_traded_price {
                    match secondary_price.compute_to_decimal(
                        last_traded_price,
                        args.order_details.side,
                        args.order_details.order_type,
                    ) {
                        Ok(v) => v,
                        Err(_) => return Err(MsgError::InvalidPrice),
                    }
                } else {
                    if secondary_price.is_relative() {
                        return Err(MsgError::NoReferencePrice);
                    } else {
                        match NonZeroDecimal::new(secondary_price.amount) {
                            Ok(v) => v,
                            Err(()) => return Err(MsgError::InvalidPrice),
                        }
                    }
                };
                Some(limit_decimal)
            } else {
                None
            };

            self.trigger_orders.push(TriggerOrderEntry {
                order_type: TriggerOrderType::TakeProfit,
                order_uuid: args.order_uuid,
                user_id: args.user_id.clone(),
                args: args.clone(),
                trigger_price: computed_price,
                limit_price,
            });

            if let TimeInForce::GoodTilDate(expiry_ts) = args.order_details.time_in_force {
                let insert_pos = self
                    .expiry_queue
                    .binary_search_by_key(&expiry_ts, |(ts, _, _)| *ts)
                    .unwrap_or_else(|pos| pos);
                self.expiry_queue
                    .insert(insert_pos, (expiry_ts, args.order_uuid, None));
            }

            return Ok(MsgOut::OrderPlaced);
        }

        let pending_fill = match matching_engine::try_fill_order::try_fill_orders(
            &mut self.orderbook,
            &args.order_details,
            computed_price,
            Some(&args.user_id),
        ) {
            Ok(v) => v,
            Err(TryFillOrdersError::ZeroQuantity) => return Err(MsgError::ZeroQuantity),
        };

        if let Err(tif_violation) = pending_fill.has_tif_violation(
            args.order_details.order_type,
            args.order_details.time_in_force,
        ) {
            pending_fill.abort();
            // if let Err(error) = transaction.rollback().await {
            //     tracing::error!(?error, "database transaction rollback error");
            // }
            return Err(MsgError::TifViolation(tif_violation));
        }

        if let Some(FillType::Cancelled) = pending_fill.taker_fill_outcome
            && args.order_details.validate_only
        {
            return Err(MsgError::OrderCancelled);
        }

        let quantity_money = Money38_18(args.order_details.quantity.unwrap().deref().clone());
        let money38_18 = match args.order_details.side {
            OrderSide::Buy => {
                if args.order_details.order_flags.volume_in_quote_currency {
                    assert_eq!(
                        args.order_details.order_type,
                        OrderType::Market,
                        "only valid for market buys"
                    );
                    quantity_money
                } else if args.order_details.order_type == OrderType::Market {
                    let (_filled_quantity, filled_cost) = pending_fill
                        .fills
                        .iter()
                        .filter(|fill| fill.fill_type != FillType::Cancelled)
                        .fold(
                            (Decimal::ZERO, Decimal::ZERO),
                            |(qty_acc, cost_acc), fill| {
                                let fill_qty = match fill.fill_type {
                                    FillType::Complete { quantity } => *quantity,
                                    FillType::Partial { amount_filled } => *amount_filled,
                                    FillType::Cancelled => Decimal::ZERO,
                                };

                                (
                                    qty_acc + fill_qty,
                                    cost_acc + (fill_qty * fill.order_index.price.deref()),
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
                        args.order_details.quantity.unwrap().deref() * computed_price.deref(),
                    )
                }
            }
            OrderSide::Sell => quantity_money.clone(),
        };

        if args.order_details.validate_only {
            return Ok(MsgOut::OrderValidated);
        }

        let (base, quote) = args.base_quote.clone();
        let currency = match args.order_details.side {
            OrderSide::Buy => quote.clone(), // to buy e.g. BTC/USD we reserve the quote asset (x btc for y usd)
            OrderSide::Sell => base.clone(), // to sell we reserve the base asset (x btc for y usd)
        };

        let balance_key = if currency == base {
            BalanceKey::Base(args.user_id)
        } else {
            BalanceKey::Quote(args.user_id)
        };

        match self.balances.reserve_money_by_asset(
            balance_key,
            money38_18,
            self.asset_pair_row
                .base_quote(&self.symbol_vocabulary)
                .unwrap(),
        ) {
            Ok(event) => events.push(event),
            Err(err) => {
                return Err(match err {
                    ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                    ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                        user_id: args.user_id,
                        asset_code: currency.clone(),
                    },
                    ReserveByAssetError::Database(error) => MsgError::BalanceDatabase(error),
                });
            }
        }

        for Fill {
            order_index,
            fill_type,
        } in &pending_fill.fills
        {
            let maker_order_data = pending_fill
                .orderbook
                .get(*order_index)
                .expect("maker order always exists in orderbook during settlement");

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
                OrderSide::Buy => (&maker_order_data.user_id, &args.user_id),
                OrderSide::Sell => (&args.user_id, &maker_order_data.user_id),
            };

            match self.balances.atomic_four_way_double_entry_transfer(
                *seller_user_id,
                Money38_18(fill_amount.deref().clone()),
                *buyer_user_id,
                Money38_18(trade_value),
                self.asset_pair_row
                    .base_quote(&self.symbol_vocabulary)
                    .expect("symbols are always resolvable"),
            ) {
                Ok(event) => events.push(event),
                Err(err) => {
                    return Err(match err {
                        SettleTransferError::InsufficientFunds {
                            user_id,
                            asset_code,
                        } => {
                            tracing::error!(
                                ?user_id,
                                ?asset_code,
                                "settlement failed: insufficient funds during trade execution"
                            );
                            MsgError::InsufficientFunds
                        }
                        SettleTransferError::AccountNotFound {
                            user_id,
                            asset_code,
                        } => {
                            tracing::error!(
                                ?user_id,
                                ?asset_code,
                                "settlement failed: account not found during trade execution"
                            );
                            MsgError::UserAccountNotFound {
                                user_id,
                                asset_code,
                            }
                        }
                    });
                }
            }
        }

        let taker_fill_outcome = pending_fill.taker_fill_outcome;
        let filled_qty = pending_fill.filled_qty();

        let new_last_trade_price = pending_fill
            .fills
            .iter()
            .filter(|fill| fill.fill_type != FillType::Cancelled)
            .map(|fill| fill.order_index.price)
            .last();

        let trade_records: Vec<(Decimal, Decimal)> = pending_fill
            .fills
            .iter()
            .filter(|fill| fill.fill_type != FillType::Cancelled)
            .map(|fill| {
                let fill_volume = match fill.fill_type {
                    FillType::Complete { quantity } => *quantity.deref(),
                    FillType::Partial { amount_filled } => *amount_filled.deref(),
                    FillType::Cancelled => Decimal::ZERO,
                };

                (*fill.order_index.price.deref(), fill_volume)
            })
            .collect();

        let fill_updates: Vec<(OrderIndex, Decimal)> = pending_fill
            .fills
            .iter()
            .filter_map(|fill| match fill.fill_type {
                FillType::Complete { quantity } => Some((fill.order_index, *quantity)),
                FillType::Partial { amount_filled } => Some((fill.order_index, *amount_filled)),
                FillType::Cancelled => None,
            })
            .collect();

        let orderbook = pending_fill.commit();

        for (order_index, fill_amount) in fill_updates {
            if let Some(maker_order) = orderbook.get_mut(order_index) {
                maker_order.filled_quantity = maker_order.filled_quantity + fill_amount;
            }
        }

        let has_resting_portion = match taker_fill_outcome {
            None => true,
            Some(FillType::Partial { .. }) => true,
            Some(FillType::Complete { .. }) | Some(FillType::Cancelled) => false,
        };

        self.last_traded_price = new_last_trade_price;

        if has_resting_portion {
            let total_qty = args
                .order_details
                .quantity
                .expect("quantity validated earlier");
            let _remaining_qty = *total_qty - filled_qty;

            let order_index = orderbook.insert(
                computed_price,
                args.order_uuid,
                args.user_id.clone(),
                args.order_details.clone(),
            );

            let open_order = OpenOrder {
                order_uuid: args.order_uuid,
                order_index,
                userref: args.order_details.userref,
                cl_ord_id: args.order_details.cl_ord_id.clone(),
                base_quote: (base, quote),
            };

            let profile = self.profiles.entry(args.user_id).or_insert(UserProfile {
                open_orders: vec![],
            });

            profile.open_orders.push(open_order);

            if let TimeInForce::GoodTilDate(expiry_ts) = args.order_details.time_in_force {
                let insert_pos = self
                    .expiry_queue
                    .binary_search_by_key(&expiry_ts, |(ts, _, _)| *ts)
                    .unwrap_or_else(|pos| pos);
                self.expiry_queue
                    .insert(insert_pos, (expiry_ts, args.order_uuid, Some(order_index)));
            }
        }

        if let Some(last_price) = self.last_traded_price {
            let mut triggered_indices = vec![];
            for (idx, entry) in self.trigger_orders.iter().enumerate() {
                let should_trigger = match entry.order_type {
                    TriggerOrderType::StopLoss => match entry.args.order_details.side {
                        OrderSide::Sell => *last_price <= *entry.trigger_price,
                        OrderSide::Buy => *last_price >= *entry.trigger_price,
                    },
                    TriggerOrderType::TakeProfit => match entry.args.order_details.side {
                        OrderSide::Sell => *last_price >= *entry.trigger_price,
                        OrderSide::Buy => *last_price <= *entry.trigger_price,
                    },
                };

                if should_trigger {
                    triggered_indices.push(idx);
                }
            }

            for idx in triggered_indices.into_iter().rev() {
                let entry = self.trigger_orders.remove(idx);

                let (base, quote) = self
                    .asset_pair_row
                    .base_quote(&self.symbol_vocabulary)
                    .expect("symbols are always resolvable");
                let (currency, reserved_amount) = {
                    let quantity = *entry.args.order_details.quantity.unwrap().deref();
                    match entry.args.order_details.side {
                        OrderSide::Buy => {
                            // Buy trigger orders reserve quote at trigger price
                            let amount = quantity * *entry.trigger_price.deref();
                            (quote.clone(), Money38_18(amount))
                        }
                        OrderSide::Sell => {
                            // Sell trigger orders reserve base quantity
                            (base.clone(), Money38_18(quantity))
                        }
                    }
                };

                let balance_key = if currency == base {
                    BalanceKey::Base(entry.user_id)
                } else {
                    BalanceKey::Quote(entry.user_id)
                };

                match self.balances.reserve_money_by_asset(
                    balance_key,
                    Money38_18(-reserved_amount.0),
                    self.asset_pair_row
                        .base_quote(&self.symbol_vocabulary)
                        .unwrap(),
                ) {
                    Ok(event) => events.push(event),
                    Err(err) => {
                        return Err(match err {
                            ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                            ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                                user_id: entry.user_id,
                                asset_code: currency.clone(),
                            },
                            ReserveByAssetError::Database(error) => {
                                MsgError::BalanceDatabase(error)
                            }
                        });
                    }
                }

                let order_type_str = match entry.order_type {
                    TriggerOrderType::StopLoss => "stop-loss",
                    TriggerOrderType::TakeProfit => "take-profit",
                };

                tracing::info!(
                    order_uuid = ?entry.order_uuid,
                    trigger_price = ?entry.trigger_price,
                    last_price = ?last_price,
                    limit_price = ?entry.limit_price,
                    "{} order triggered",
                    order_type_str
                );

                let mut triggered_args = entry.args.clone();
                if let Some(limit_price) = entry.limit_price {
                    triggered_args.order_details.order_type = OrderType::Limit;
                    triggered_args.order_details.price = Price {
                        prefix: None,
                        amount: *limit_price.deref(),
                        is_percentage: false,
                    };
                } else {
                    triggered_args.order_details.order_type = OrderType::Market;
                    triggered_args.order_details.price = Price {
                        prefix: None,
                        amount: 0.into(),
                        is_percentage: false,
                    };
                }

                let (_result, triggered_events) =
                    self.switch_msg_in(MsgIn::PlaceOrder(triggered_args));
                events.extend(triggered_events);
            }
        };

        for (price, fill_volume) in trade_records {
            self.record_trade(price, fill_volume, now_time);
        }

        Ok(MsgOut::OrderPlaced)
    }

    fn cancel_order_by(
        &mut self,
        CancelOrderByArgs {
            user_id,
            cancel_order_by,
        }: CancelOrderByArgs,
        events: &mut Vec<BroadcastEvent>,
    ) -> SwitchMsgInOutput {
        let (base, quote) = self
            .asset_pair_row
            .base_quote(&self.symbol_vocabulary)
            .expect("symbols are always resolvable");

        let mut trigger_to_cancel = vec![];
        for (idx, entry) in self.trigger_orders.iter().enumerate() {
            if entry.user_id != user_id {
                continue;
            }

            let should_cancel = match &cancel_order_by {
                CancelOrderBy::TxId(order_uuid) => entry.order_uuid == *order_uuid,
                CancelOrderBy::Userref(userref) => {
                    entry.args.order_details.userref == Some(*userref)
                }
                CancelOrderBy::ClientOrderId(cl_ord_id) => entry
                    .args
                    .order_details
                    .cl_ord_id
                    .as_ref()
                    .map(|id| id == cl_ord_id)
                    .unwrap_or(false),
            };

            if should_cancel {
                trigger_to_cancel.push(idx);
            }
        }

        let mut success = vec![];

        for idx in trigger_to_cancel.into_iter().rev() {
            let entry = self.trigger_orders.remove(idx);

            let (currency, refund_amount) = {
                let quantity = *entry.args.order_details.quantity.unwrap().deref();
                match entry.args.order_details.side {
                    OrderSide::Buy => {
                        // For buy orders, refund quantity * trigger_price in quote currency
                        let amount = quantity * *entry.trigger_price.deref();
                        (quote.clone(), Money38_18(amount))
                    }
                    OrderSide::Sell => {
                        // For sell orders, refund quantity in base currency
                        (base.clone(), Money38_18(quantity))
                    }
                }
            };

            let balance_key = if currency == base {
                BalanceKey::Base(user_id)
            } else {
                BalanceKey::Quote(user_id)
            };

            debug_assert!(
                refund_amount.0 > Decimal::ZERO,
                "refund amount must be positive"
            );
            match self.balances.reserve_money_by_asset(
                balance_key,
                Money38_18(-refund_amount.0),
                self.asset_pair_row
                    .base_quote(&self.symbol_vocabulary)
                    .unwrap(),
            ) {
                Ok(event) => events.push(event),
                Err(err) => {
                    return Err(match err {
                        ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                        ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                            user_id,
                            asset_code: currency.clone(),
                        },
                        ReserveByAssetError::Database(error) => MsgError::BalanceDatabase(error),
                    });
                }
            }

            self.expiry_queue
                .retain(|(_, uuid, _)| *uuid != entry.order_uuid);

            success.push(entry.order_uuid);
        }

        if !success.is_empty() {
            return Ok(MsgOut::OrderCancelled {
                success,
                failed: vec![],
            });
        }

        let orders_to_cancel: Vec<_> =
            match self.profiles.get(&user_id).ok_or(MsgError::NoOpenPositions) {
                Ok(profile) => profile.isolate_orders_for_cancel(&cancel_order_by),
                Err(e) => return Err(e),
            };

        if orders_to_cancel.is_empty() {
            return Err(MsgError::OrderNotFound);
        }

        let mut success = vec![];
        let failed = vec![]; // TODO: right now failures arent a thing we either panic or yeet if we cant cancel/refund positions.

        for open_order in &orders_to_cancel {
            let order_index = open_order.order_index.clone();
            let (remaining_quantity, price, order_id) = {
                let order_data = self
                    .orderbook
                    .get(order_index.clone())
                    .expect("always valid");
                (
                    order_data.remaining_quantity,
                    order_data.price,
                    order_data.order_id.clone(),
                )
            };

            // TODO: are Iceberg orders handled?
            let (currency, reserved_amount) = match order_index.side {
                OrderSide::Buy => {
                    // Buy orders reserve quote currency (quantity * price)
                    let amount = *remaining_quantity * *price;
                    (quote.clone(), Money38_18(amount))
                }
                OrderSide::Sell => {
                    // Sell orders reserve base currency (quantity)
                    let amount = *remaining_quantity;
                    (base.clone(), Money38_18(amount))
                }
            };

            let balance_key = if currency == base {
                BalanceKey::Base(user_id)
            } else {
                BalanceKey::Quote(user_id)
            };

            debug_assert!(
                reserved_amount.0 > Decimal::ZERO,
                "reserved amount must be positive"
            );
            match self.balances.reserve_money_by_asset(
                balance_key,
                Money38_18(-reserved_amount.0),
                self.asset_pair_row
                    .base_quote(&self.symbol_vocabulary)
                    .unwrap(),
            ) {
                Ok(event) => events.push(event),
                Err(err) => {
                    return Err(match err {
                        ReserveByAssetError::InsufficientFunds => MsgError::InsufficientFunds,
                        ReserveByAssetError::AccountNotFound => MsgError::UserAccountNotFound {
                            user_id,
                            asset_code: currency.clone(),
                        },
                        ReserveByAssetError::Database(error) => MsgError::BalanceDatabase(error),
                    });
                }
            }

            success.push(order_id);
        }

        for open_order in &orders_to_cancel {
            self.expiry_queue
                .retain(|(_, uuid, _)| *uuid != open_order.order_uuid);
        }

        let profile = self.profiles.get_mut(&user_id).expect("user exists");

        for open_order in &orders_to_cancel {
            let order_index = open_order.order_index.clone();
            let _order_data = self
                .orderbook
                .remove(order_index.clone())
                .expect("always in");
        }

        for order in profile.open_orders.extract_if(.., |o| {
            orders_to_cancel
                .iter()
                .find(|t| t.order_uuid == o.order_uuid)
                .is_some()
        }) {
            tracing::trace!(?order, "cancelling order");
        }

        Ok(MsgOut::OrderCancelled { success, failed })
    }

    async fn persist_events(&self, events: Vec<BroadcastEvent>) {
        if events.is_empty() {
            return;
        }

        let mut transaction = self.pg_pool.begin().await.unwrap();

        for event in events {
            tracing::trace!(?event, "broadcasting event");

            match event {
                BroadcastEvent::Reserve {
                    user_id,
                    amount,
                    asset_code,
                } => {
                    if amount == Decimal::ZERO {
                        continue;
                    }

                    let user_account_id = sqlx::query!(
                        r#"
                            SELECT id
                            FROM t_money_accounts
                            WHERE user_id = $1
                              AND currency = $2
                        "#,
                        user_id,
                        asset_code.as_str(),
                    )
                    .fetch_one(transaction.deref_mut())
                    .await
                    .expect("user money account must exist before reserving")
                    .id;

                    let exchange_account_id = if let Some(record) = sqlx::query!(
                        r#"
                            SELECT id
                            FROM t_money_accounts
                            WHERE currency = $1
                              AND fiat_source IS NOT NULL
                            LIMIT 1
                        "#,
                        asset_code.as_str(),
                    )
                    .fetch_optional(transaction.deref_mut())
                    .await
                    .expect("exchange fiat account lookup must succeed")
                    {
                        record.id
                    } else {
                        sqlx::query!(
                            r#"
                                SELECT id
                                FROM t_money_accounts
                                WHERE currency = $1
                                  AND crypto_source IS NOT NULL
                                LIMIT 1
                            "#,
                            asset_code.as_str(),
                        )
                        .fetch_one(transaction.deref_mut())
                        .await
                        .expect("exchange crypto account lookup must succeed")
                        .id
                    };

                    if amount > Decimal::ZERO {
                        sqlx::query!(
                            r#"
                                INSERT INTO t_account_tx_journal
                                    (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                                VALUES ($1, $2, $3, $4::numeric, 'reserve asset', $5)
                            "#,
                            exchange_account_id,
                            user_account_id,
                            asset_code.as_str(),
                            amount,
                            uuid::Uuid::new_v4().to_string(),
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .expect("reserving funds must succeed");
                    } else {
                        sqlx::query!(
                            r#"
                                INSERT INTO t_account_tx_journal
                                    (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                                VALUES ($1, $2, $3, $4::numeric, 'cancel_refund', $5)
                            "#,
                            user_account_id,
                            exchange_account_id,
                            asset_code.as_str(),
                            amount.abs(),
                            uuid::Uuid::new_v4().to_string(),
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .expect("refund must succeed");
                    }
                }
                BroadcastEvent::Settlement {
                    buyer_id,
                    seller_id,
                    base_amount,
                    quote_amount,
                    base_asset,
                    quote_asset,
                } => {
                    if base_amount > Decimal::ZERO {
                        let buyer_account_id = sqlx::query!(
                            r#"
                                SELECT id
                                FROM t_money_accounts
                                WHERE user_id = $1
                                  AND currency = $2
                            "#,
                            buyer_id,
                            base_asset.as_str(),
                        )
                        .fetch_one(transaction.deref_mut())
                        .await
                        .expect("buyer base account must exist")
                        .id;

                        let exchange_base_account_id = if let Some(record) = sqlx::query!(
                            r#"
                                SELECT id
                                FROM t_money_accounts
                                WHERE currency = $1
                                  AND fiat_source IS NOT NULL
                                LIMIT 1
                            "#,
                            base_asset.as_str(),
                        )
                        .fetch_optional(transaction.deref_mut())
                        .await
                        .expect("exchange base fiat account lookup must succeed")
                        {
                            record.id
                        } else {
                            sqlx::query!(
                                r#"
                                    SELECT id
                                    FROM t_money_accounts
                                    WHERE currency = $1
                                      AND crypto_source IS NOT NULL
                                    LIMIT 1
                                "#,
                                base_asset.as_str(),
                            )
                            .fetch_one(transaction.deref_mut())
                            .await
                            .expect("exchange base crypto account lookup must succeed")
                            .id
                        };

                        sqlx::query!(
                            r#"
                                INSERT INTO t_account_tx_journal
                                    (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                                VALUES ($1, $2, $3, $4::numeric, 'trade_settlement', $5)
                            "#,
                            buyer_account_id,
                            exchange_base_account_id,
                            base_asset.as_str(),
                            base_amount,
                            uuid::Uuid::new_v4().to_string(),
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .expect("base settlement must succeed");
                    }

                    if quote_amount > Decimal::ZERO {
                        let seller_account_id = sqlx::query!(
                            r#"
                                SELECT id
                                FROM t_money_accounts
                                WHERE user_id = $1
                                  AND currency = $2
                            "#,
                            seller_id,
                            quote_asset.as_str(),
                        )
                        .fetch_one(transaction.deref_mut())
                        .await
                        .expect("seller quote account must exist")
                        .id;

                        let exchange_quote_account_id = if let Some(record) = sqlx::query!(
                            r#"
                                SELECT id
                                FROM t_money_accounts
                                WHERE currency = $1
                                  AND fiat_source IS NOT NULL
                                LIMIT 1
                            "#,
                            quote_asset.as_str(),
                        )
                        .fetch_optional(transaction.deref_mut())
                        .await
                        .expect("exchange quote fiat account lookup must succeed")
                        {
                            record.id
                        } else {
                            sqlx::query!(
                                r#"
                                    SELECT id
                                    FROM t_money_accounts
                                    WHERE currency = $1
                                      AND crypto_source IS NOT NULL
                                    LIMIT 1
                                "#,
                                quote_asset.as_str(),
                            )
                            .fetch_one(transaction.deref_mut())
                            .await
                            .expect("exchange quote crypto account lookup must succeed")
                            .id
                        };

                        sqlx::query!(
                            r#"
                                INSERT INTO t_account_tx_journal
                                    (credit_account_id, debit_account_id, currency, amount, transaction_type, txid)
                                VALUES ($1, $2, $3, $4::numeric, 'trade_settlement', $5)
                            "#,
                            seller_account_id,
                            exchange_quote_account_id,
                            quote_asset.as_str(),
                            quote_amount,
                            uuid::Uuid::new_v4().to_string(),
                        )
                        .execute(transaction.deref_mut())
                        .await
                        .expect("quote settlement must succeed");
                    }
                }
            }
        }

        transaction.commit().await.unwrap();
    }
}

async fn insert_into_t_trading_event_source(proc: &Proc, msg_in: &MsgIn) -> Result<(), MsgError> {
    // Skip persisting Shutdown or ticker snapshot messages to event source
    if matches!(msg_in, MsgIn::Shutdown | MsgIn::TickerSnapshot) {
        return Ok(());
    }

    let base_quote = proc
        .asset_pair_row
        .base_quote(&proc.symbol_vocabulary)
        .expect("symbols are always resolvable");

    let value = serde_json::to_value(&msg_in).map_err(|err| MsgError::UnserializableInput {
        base_quote: base_quote.clone(),
        message: msg_in.to_owned(),
        serde_json_error: err,
    })?;

    let pg_query_result = sqlx::query!(
        "INSERT INTO t_trading_event_source (jstr, base_asset, quote_asset) VALUES ($1, $2, $3)",
        value,
        base_quote.0.as_str(),
        base_quote.1.as_str()
    )
    .execute(&proc.pg_pool)
    .await
    .map_err(|err| MsgError::CouldNotPersistToEventSource(err))?;

    assert_eq!(pg_query_result.rows_affected(), 1);

    Ok(())
}

async fn ap_loop_select(mut mpsc_receiver: mpsc::Receiver<Envelope>, mut proc: Proc) {
    enum Event {
        MsgIn(Option<Envelope>),
        PgEvent(Result<PgNotification, sqlx::Error>),
        ExpiryReached,
    }

    // let (s, mut pg_listener) = mpsc::channel(1);
    // tokio::task::spawn({
    let pg_pool = proc.pg_pool.clone();
    //     async move {
    let mut pg_listener = PgListener::connect_with(&pg_pool).await.unwrap();
    pg_listener.listen("account_tx_journal").await.unwrap();
    //         loop {
    //             let _ = s.send(pg_listener.recv().await).await;
    //         }
    //     }
    // });

    while !proc.will_shutdown {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expiry_sleep = match proc.expiry_queue.first() {
            Some((expiry_ts, _, _)) if *expiry_ts <= now => {
                let mut events = vec![];
                proc.handle_expiry(&mut events);
                proc.persist_events(events).await;
                continue;
            }
            None => tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)),
            Some((expiry_ts, _, _)) => {
                tokio::time::sleep(std::time::Duration::from_secs(expiry_ts - now))
            }
        };

        let event = tokio::select! {
            t = mpsc_receiver.recv() => Event::MsgIn(t),
            n = pg_listener.recv() => Event::PgEvent(n),
            _ = expiry_sleep => Event::ExpiryReached,
        };

        match event {
            Event::MsgIn(None) => {
                // Channel closed - all senders dropped (e.g., tests finished, server shutdown)
                proc.will_shutdown = true;
            }
            Event::MsgIn(Some((snd, msg_in))) => {
                let (response, events) =
                    match insert_into_t_trading_event_source(&proc, &msg_in).await {
                        Err(err) => (Err(err), vec![]),
                        Ok(_) => proc.switch_msg_in(msg_in),
                    };

                proc.persist_events(events).await;

                if let Err(_response) = snd.send(response) {
                    tracing::warn!("original message requestor droppped reciever for response")
                }
            }
            Event::PgEvent(event) => {
                tracing::error!("unimplemented! account_tx_journal {event:#?}")
            }
            Event::ExpiryReached => {
                let mut events = vec![];
                proc.handle_expiry(&mut events);
                proc.persist_events(events).await;
            }
        }
    }
}

#[derive(Debug)]
pub struct ProcHandle {
    pub base_quote: BaseQuote,
    pub mpsc_sender: mpsc::Sender<Envelope>,
    pub broadcast_sender: broadcast::WeakSender<BroadcastEvent>,
    pub asset_pair_row: AssetPairRow,
    #[allow(dead_code)]
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

pub async fn launch_processors_for_pairs<'a>(
    t_trading_asset_pairs: Vec<AssetPairRow>,
    pg_pool: sqlx::PgPool,
) -> anyhow::Result<(SymbolVocabulary, Vec<ProcHandle>)> {
    let symbol_vocabulary = t_trading_asset_pairs
        .iter()
        .map(|r| [r.base_asset.clone(), r.quote_asset.clone()])
        .flatten()
        .unique()
        .collect::<SymbolVocabulary>();

    let channel_buffer_size = option_env!("AP_CHANNEL_BUFFER_SIZE") // TODO: can we do better?
        .and_then(|st| {
            st.parse()
                .inspect_err(|err| {
                    tracing::error!(?err, ?st, "The COMPILE TIME environment variable AP_CHANNEL_BUFFER_SIZE could not be parsed to a number (usize)");
                })
                .ok()
        })
        .unwrap_or(1);

    let t_money_accounts = crate::money_accounts::money_account_balances(pg_pool.clone()).await;

    let ap_info = futures::stream::iter(t_trading_asset_pairs.into_iter())
        .map(|asset_pair_row| {
            let symbol_vocabulary = symbol_vocabulary.clone();
            let pg_pool = pg_pool.clone();
            let t_money_accounts = &t_money_accounts;
            async move {
                let balances = t_money_accounts
                    .values()
                    .filter_map(|money_account| {
                        let user_id = money_account.user_id?;

                        let key = if money_account.currency == asset_pair_row.base_asset {
                            BalanceKey::Base(user_id)
                        } else if money_account.currency == asset_pair_row.quote_asset {
                            BalanceKey::Quote(user_id)
                        } else {
                            return None;
                        };

                        Some((key, money_account.balance.clone()))
                    })
                    .collect();

                let mut proc = Proc {
                    pg_pool: pg_pool.clone(),
                    orderbook: Orderbook::new_empty(),
                    profiles: Default::default(),
                    balances: Balances(balances),
                    asset_pair_row: asset_pair_row.clone(),
                    symbol_vocabulary: symbol_vocabulary.clone(),
                    expiry_queue: vec![],
                    trigger_orders: vec![],
                    last_traded_price: None,
                    last_trade_volume: None,
                    ticker_trades: VecDeque::new(),
                    status: ProcStatus::Online,
                    will_shutdown: false,
                    // start_time: (Instant::now(), SystemTime::now()),
                };

                let t_trading_event_source = sqlx::query!(
                    r#"SELECT id,jstr FROM t_trading_event_source where base_asset = $1 AND quote_asset = $2"#,
                    asset_pair_row.base_asset,
                    asset_pair_row.quote_asset
                )
                .fetch(&pg_pool);

                tokio::pin!(t_trading_event_source);
                while let Some(maybe_row) = t_trading_event_source.next().await {
                    let record = match maybe_row {
                        Ok(record) => record,
                        Err(error) => {
                            tracing::error!(?error, "error fetching trading event source row");
                            panic!("error fetching trading event source row");
                        }
                    };

                    tracing::trace!(?record.id, "processing trading event source row");
                    let Ok(msg_in) = serde_json::from_value::<MsgIn>(record.jstr) else {
                        tracing::error!(?record.id, "error deserializing trading event source row");
                        panic!("error deserializing trading event source row");
                    };

                    let _ = proc.switch_msg_in(msg_in);
                }

                let (mpsc_sender, mpsc_receiver) = tokio::sync::mpsc::channel(channel_buffer_size);
                let (broadcast_sender, _) = broadcast::channel(channel_buffer_size);

                let base_quote = proc
                    .asset_pair_row
                    .base_quote(&symbol_vocabulary)
                    .expect("base or quote was not in symbol vocabulary");

                let span = tracing::info_span!(
                    "asset_processor_loop",
                    id = proc.asset_pair_row.id,
                    base = asset_pair_row.base_asset,
                    quote = asset_pair_row.quote_asset,
                );
                let join_handle = tokio::task::spawn(ap_loop_select(mpsc_receiver, proc).instrument(span));

                ProcHandle {
                    base_quote,
                    mpsc_sender,
                    broadcast_sender: broadcast_sender.downgrade(),
                    join_handle: Some(join_handle),
                    asset_pair_row
                }
            }
        })
        .buffer_unordered(1)
        .collect()
        .await;

    Ok((symbol_vocabulary, ap_info))
}
