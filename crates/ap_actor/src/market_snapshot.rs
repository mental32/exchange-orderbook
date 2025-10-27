use matching_engine::decimal::Decimal;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::orderbook::Orderbook;
use matching_engine::pending_fill::Fill;
use matching_engine::pending_fill::FillType;
use std::collections::VecDeque;
use std::ops::Deref as _;

pub const MARKET_DEPTH_LEVEL_LIMIT: usize = 50;
pub const MARKET_TRADES_LIMIT: usize = 512;
pub const MARKET_SPREAD_LIMIT: usize = 512;

pub fn trade_items_from_fills(
    fills: &[Fill],
    taker_side: OrderSide,
    order_type: OrderType,
    timestamp: u64,
) -> Vec<TradeItem> {
    let mut trades = Vec::new();
    for fill in fills {
        let volume = match fill.fill_type {
            FillType::Complete { quantity } => *quantity.deref(),
            FillType::Partial { amount_filled } => *amount_filled.deref(),
            FillType::Cancelled => continue,
        };

        trades.push(TradeItem {
            trade_id: 0,
            price: *fill.order_index.price.deref(),
            volume,
            taker_side,
            order_type,
            timestamp,
        });
    }

    trades
}

#[derive(Debug, Clone, Default)]
pub struct DepthLevel {
    pub price: Decimal,
    pub volume: Decimal,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct TradeItem {
    pub trade_id: u64,
    pub price: Decimal,
    pub volume: Decimal,
    pub taker_side: OrderSide,
    pub order_type: OrderType,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SpreadSample {
    pub timestamp: u64,
    pub bid_price: Decimal,
    pub bid_volume: Decimal,
    pub ask_price: Decimal,
    pub ask_volume: Decimal,
}

#[derive(Debug, Clone, Default)]
pub struct MarketSnapshot {
    pub last_update: u64,
    pub best_bid: Option<DepthLevel>,
    pub best_ask: Option<DepthLevel>,
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub trades: VecDeque<TradeItem>,
    pub spread: VecDeque<SpreadSample>,
}

impl MarketSnapshot {
    pub fn append_trades(&mut self, mut trades: Vec<TradeItem>) {
        if trades.is_empty() {
            return;
        }
        for trade in trades.drain(..) {
            self.trades.push_back(trade);
        }

        while self.trades.len() > MARKET_TRADES_LIMIT {
            self.trades.pop_front();
        }
    }

    pub fn append_spread(&mut self, sample: SpreadSample) {
        if let Some(last) = self.spread.back() {
            if last.bid_price == sample.bid_price
                && last.ask_price == sample.ask_price
                && last.bid_volume == sample.bid_volume
                && last.ask_volume == sample.ask_volume
            {
                return;
            }
        }

        self.spread.push_back(sample);
        while self.spread.len() > MARKET_SPREAD_LIMIT {
            self.spread.pop_front();
        }
    }

    pub fn rebuild_depth<T>(&mut self, orderbook: &Orderbook<T>, timestamp: u64) {
        self.bids.clear();
        self.asks.clear();

        for (_, order) in orderbook.bids() {
            let price = *order.price.deref();
            let volume = *order.remaining_quantity.deref();

            match self.bids.last_mut() {
                Some(last) if last.price == price => {
                    last.volume += volume;
                }
                _ => {
                    if self.bids.len() == MARKET_DEPTH_LEVEL_LIMIT {
                        break;
                    }
                    self.bids.push(DepthLevel {
                        price,
                        volume,
                        timestamp,
                    });
                }
            }
        }

        for (_, order) in orderbook.asks() {
            let price = *order.price.deref();
            let volume = *order.remaining_quantity.deref();

            match self.asks.last_mut() {
                Some(last) if last.price == price => {
                    last.volume += volume;
                }
                _ => {
                    if self.asks.len() == MARKET_DEPTH_LEVEL_LIMIT {
                        break;
                    }
                    self.asks.push(DepthLevel {
                        price,
                        volume,
                        timestamp,
                    });
                }
            }
        }

        self.best_bid = self.bids.first().cloned();
        self.best_ask = self.asks.first().cloned();
    }
}
