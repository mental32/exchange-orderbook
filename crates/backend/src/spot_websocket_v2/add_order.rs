use matching_engine::decimal::Decimal;
use matching_engine::orderbook::OrderSide;
use matching_engine::orderbook::OrderType;
use matching_engine::orderbook::SelfTradeProtection;
use matching_engine::orderbook::TimeInForce;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FeePreference {
    Base,
    Quote,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Triggers {
    /// The reference price to track for triggering orders.
    /// *   `index`: the index price in the broader market (for this pair). Note, to keep triggers serviceable during connectivity issues with external index feeds, the last price will be used as the reference price.
    /// *   `last`: the last traded price in the order book (for this pair).
    reference: PriceTrigger,
    /// Specifies the amount for the trigger price - it supports both static market prices and relative prices. This field is used in combination with the price_type field below to determine the effective trigger price.
    /// Examples:
    /// -   To trigger at 29000.5 BTC/USD, use price=29000.5, price_type=static.
    /// -   To trigger when price rises by 5%, use price=5, price_type=pct.
    /// -   To trigger when price drops by 150 USD, use price=-150, price_type=quote.
    price: f64, // Required
    /// Default value: static
    #[cfg_attr(feature = "serde", serde(default = "PriceType::statik"))]
    price_type: PriceType,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PriceType {
    #[cfg_attr(feature = "serde", serde(rename = "static"))]
    Static,
    #[cfg_attr(feature = "serde", serde(rename = "pct"))]
    Percentage,
    #[cfg_attr(feature = "serde", serde(rename = "quote"))]
    Quote,
}

impl PriceType {
    fn statik() -> Self {
        PriceType::Static
    }
}

impl Default for PriceType {
    fn default() -> Self {
        PriceType::Quote
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PriceTrigger {
    /// Index price is derived from the sum of the prices from various spot exchanges multiplied by their respective weighage.
    Index,
    /// This is the platform's current market price.
    Last,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Conditional {
    /// Defines the order type of the secondary close orders which will be created on each fill.
    pub order_type: OrderType,
    #[cfg_attr(
        feature = "serde",
        serde(with = "matching_engine::decimal::serde::str")
    )]
    pub limit_price: Decimal,
    pub limit_price_type: PriceType,
    #[cfg_attr(
        feature = "serde",
        serde(with = "matching_engine::decimal::serde::str")
    )]
    pub trigger_price: Decimal,
    pub trigger_price_type: PriceType,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AddOrder {
    /// The execution model of the order.
    /// *   market: The full order quantity executes immediately at the best available price in the order book.
    /// *   limit: The full order quantity is placed immediately with a limit price restriction to only trade at this price or better.
    /// *   stop-loss: A market order is triggered when the reference price reaches the stop price (from an unfavourable direction).
    /// *   stop-loss-limit: A limit order is triggered when the reference price reaches the stop price (from an unfavourable direction).
    /// *   take-profit: A market order is triggered when the reference price reaches the stop price (from an favourable direction).
    /// *   take-profit-limit: A limit order is triggered when the reference price reaches the stop price (from an favourable direction).
    /// *   trailing-stop: A market order is triggered when the market reverts a specified distance from the peak price.
    /// *   trailing-stop-limit: A limit order is triggered when the market reverts a specified distance from the peak price.
    /// *   iceberg: Hides the full order size by only showing your chosen display size in the book at your limit price.
    pub order_type: OrderType, // Required
    /// Side of the order.
    pub side: OrderSide, // Required
    /// Order quantity in terms of the base asset.
    #[cfg_attr(
        feature = "serde",
        serde(with = "matching_engine::decimal::serde::str")
    )]
    #[cfg_attr(feature = "serde", serde(rename = "order_qty"))]
    pub quantity: Decimal, // Required
    /// The symbol of the currency pair.
    /// Exaple: "BTC/USD"
    pub symbol: String, // Required
    /// Limit price for order types that support limit price restriction.
    pub limit_price: Option<f64>,
    /// The units for the limit price:
    ///     -   `static`: a static market price for the asset, i.e.,  30000 for BTC/USD.
    ///     -   `pct`: a percentage offset from the reference price i.e. -10% from index price.
    ///     -   `quote`: a notional offset from the reference price in the quote currency, i.e., 150 BTC/USD from last price.
    ///
    /// Note, from `trailing-stop-limit` order type, the value represents the offset from the trigger price. 0
    /// would set a limit price same as the trigger price.
    ///
    /// Condition: Only available on trailing-stop-limit orders.
    /// Default value: quote
    #[cfg_attr(feature = "serde", serde(default))]
    pub limit_price_type: PriceType, // Conditional
    /// The parameters for setting the trigger price conditions.
    /// Condition: Required for triggered order types only.
    pub triggers: Triggers, // Conditional
    /// The time in force for the order.
    /// Default value: good_till_canceled
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_in_force: TimeInForce,
    /// Funds the order on margin using the maximum leverage for the pair (maximum is leverage of 5).
    #[cfg_attr(feature = "serde", serde(default))]
    pub margin: bool,
    /// Cancels the order if it will take liquidity on arrival. Post only orders will always be posted passively in the book.
    #[cfg_attr(feature = "serde", serde(default))]
    pub post_only: bool,
    /// Reduces an existing margin position without opening an opposite long or short position worth more than the current value of your leveraged assets.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reduce_only: bool,
    /// Scheduled start time (precision to seconds).
    /// Format: RFC3339
    /// Example: 2022-12-25T09:30:59Z
    pub effective_time: String,
    /// Expiration time of the order (precision to seconds). GTD orders can have an expiry time up to one
    /// month in future.
    /// Condition: GTD orders only.
    pub expire_time: String, // Conditional
    /// Range of valid offsets (from current time) is 500 milliseconds to 60 seconds, default is 5 seconds.
    /// The precision of this parameter is to the millisecond. The engine will prevent this order from
    /// matching after this time, it provides protection against latency on time sensitive orders.
    pub deadline: String,
    /// Adds an alphanumeric client order identifier which uniquely identifies an open order for each client.
    /// This field is mutually exclusive with `userref` parameter.
    /// The `cl_ord_id` parameter can be one of the following formats:
    /// -   Long UUID: `6d1b345e-2821-40e2-ad83-4ecb18a06876` 32 hex characters separated with 4 dashes.
    /// -   Short UUID: `da8e4ad59b78481c93e589746b0cf91f` 32 hex characters with no dashes.
    /// -   Free text: `arb-20230101-123456` free format ascii text up to 18 characters.
    pub cl_ord_id: Option<String>,
    /// The conditional parameters are used as a template for generating the secondary close orders when the
    /// primary order fills. Each fill on the primary order will generate a new secondary order. The size of
    /// the secondary order will be the same size as the executed quantity and have the opposite side.
    pub conditional: Conditional,
    /// Defines the quantity to show in the book while the rest of order quantity remains hidden.
    #[cfg_attr(feature = "serde", serde(with = "rust_decimal::serde::str"))]
    #[cfg_attr(feature = "serde", serde(rename = "display_qty"))]
    pub display_quantity: Decimal,
    /// Fee preference base or quote currency. quote is the default for buy orders, base is the default for
    /// sell orders.
    pub fee_preference: FeePreference,
    /// Self Trade Prevention (STP) is a protection feature to prevent users from inadvertently or
    /// deliberately trading against themselves. To prevent a self-match, one of the following STP modes can
    /// be used to define which order(s) will be expired:
    /// *   `cancel_newest`: arriving order will be canceled.
    /// *   `cancel_oldest`: oldest order will be canceled.
    /// *   `cancel_both`: both orders will be canceled.
    #[cfg_attr(feature = "serde", serde(default, rename = "stp_type"))]
    pub self_trade_protection: SelfTradeProtection,
    /// Order volume expressed in quote currency.
    pub cash_order_qty: f64, // Conditional
    /// If set to true the order will be validated only, it will not trade in the matching engine.
    #[cfg_attr(feature = "serde", serde(default))]
    pub validate: bool,
    /// Adds a alphanumeric sub-account/trader identifier which enables STP to be performed at a more granular level.
    /// The sender_sub_id parameter can be one of the following formats:
    /// *   Long UUID: 6d1b345e-2821-40e2-ad83-4ecb18a06876 32 hex characters separated with 4 dashes.
    /// *   Short UUID: da8e4ad59b78481c93e589746b0cf91f 32 hex characters with no dashes.
    /// *   Free text: arb-20240509-00010 Free format ascii text up to 18 characters.
    /// Condition: For institutional accounts with enhanced Self Trade Prevention (STP)
    pub sender_sub_id: String, // Conditional
    pub token: String,
}
