use crate::asset_pair::BaseQuote;
use crate::decimal::Decimal;
use crate::decimal::NonZeroDecimal;
use crate::orderbook::OrderSide;
use crate::orderbook::OrderType;
use crate::orderbook::SelfTradeProtection;
use crate::orderbook::TimeInForce;
use crate::orderflags::OrderFlags;
use crate::price::Price;
use crate::time::Time;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrderTicket {
    pub order_type: OrderType,
    pub side: OrderSide,
    pub quantity: Option<NonZeroDecimal>,
    // pub symbol: BaseQuote,
    #[serde(with = "crate::price")]
    pub price: Price,
    // pub limit_price: Option<f64>,
    // pub limit_price_type: PriceType,
    pub time_in_force: TimeInForce,
    pub stp: SelfTradeProtection,
    pub display_quantity: Option<NonZeroDecimal>,
    pub userref: Option<u32>,
    // pub margin: bool,
    // pub post_only: bool,
    // pub reduce_only: bool,
    // pub effective_time: String,
    // pub expire_time: String,
    // pub deadline: String,
    pub cl_ord_id: Option<String>,
    #[serde(with = "crate::time")]
    pub expiry_time: Time,
    pub volume: Decimal,
    #[serde(
        default,
        deserialize_with = "crate::price::deserialize_price_option",
        serialize_with = "crate::price::serialize_price_option"
    )]
    pub secondary_price: Option<Price>,
    #[serde(default, with = "crate::orderflags")]
    pub order_flags: OrderFlags,
    // pub fee_preference: FeePreference,
    // pub self_trade_protection: SelfTradeProtection,
    pub validate_only: bool,
    // pub sender_sub_id: String,
}

#[derive(Debug)]
pub struct OrderTicketBuilder {
    order_type: OrderType,
    order_side: OrderSide,
    quantity: Option<NonZeroDecimal>,
    price: Option<Price>,
    time_in_force: Option<TimeInForce>,
    stp: Option<SelfTradeProtection>,
    display_quantity: Option<NonZeroDecimal>,
    userref: Option<u32>,
    cl_ord_id: Option<String>,
    expiry_time: Option<Time>,
    volume: Option<Decimal>,
    secondary_price: Option<Price>,
    order_flags: Option<OrderFlags>,
    validate_only: Option<bool>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrderTicketBuilderError {
    #[error("price must be specified for order ticket")]
    MissingPrice,
}

impl OrderTicketBuilder {
    pub fn new(order_type: OrderType, order_side: OrderSide) -> Self {
        Self {
            order_type,
            order_side,
            quantity: None,
            price: None,
            time_in_force: None,
            stp: None,
            display_quantity: None,
            userref: None,
            cl_ord_id: None,
            expiry_time: None,
            volume: None,
            secondary_price: None,
            order_flags: None,
            validate_only: None,
        }
    }

    pub fn build(self) -> Result<OrderTicket, OrderTicketBuilderError> {
        let price = self.price.ok_or(OrderTicketBuilderError::MissingPrice)?;

        Ok(OrderTicket {
            order_type: self.order_type,
            side: self.order_side,
            quantity: self.quantity,
            price,
            time_in_force: self.time_in_force.unwrap_or_default(),
            stp: self.stp.unwrap_or_default(),
            display_quantity: self.display_quantity,
            userref: self.userref,
            cl_ord_id: self.cl_ord_id,
            expiry_time: self.expiry_time.unwrap_or_default(),
            volume: self.volume.unwrap_or_default(),
            secondary_price: self.secondary_price,
            order_flags: self.order_flags.unwrap_or_default(),
            validate_only: self.validate_only.unwrap_or_default(),
        })
    }

    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = order_type;
        self
    }

    pub fn side(mut self, side: OrderSide) -> Self {
        self.order_side = side;
        self
    }

    pub fn quantity(mut self, quantity: NonZeroDecimal) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    pub fn time_in_force(mut self, time_in_force: TimeInForce) -> Self {
        self.time_in_force = Some(time_in_force);
        self
    }

    pub fn stp(mut self, stp: SelfTradeProtection) -> Self {
        self.stp = Some(stp);
        self
    }

    pub fn display_quantity(mut self, display_quantity: NonZeroDecimal) -> Self {
        self.display_quantity = Some(display_quantity);
        self
    }

    pub fn userref(mut self, userref: u32) -> Self {
        self.userref = Some(userref);
        self
    }

    pub fn cl_ord_id(mut self, cl_ord_id: String) -> Self {
        self.cl_ord_id = Some(cl_ord_id);
        self
    }

    pub fn expiry_time(mut self, expiry_time: Time) -> Self {
        self.expiry_time = Some(expiry_time);
        self
    }

    pub fn volume(mut self, volume: Decimal) -> Self {
        self.volume = Some(volume);
        self
    }

    pub fn secondary_price(mut self, secondary_price: Price) -> Self {
        self.secondary_price = Some(secondary_price);
        self
    }

    pub fn order_flags(mut self, order_flags: OrderFlags) -> Self {
        self.order_flags = Some(order_flags);
        self
    }

    pub fn validate_only(mut self, validate_only: bool) -> Self {
        self.validate_only = Some(validate_only);
        self
    }
}

impl OrderTicket {
    pub fn builder(
        order_type: OrderType,
        order_side: OrderSide,
        price: Price,
    ) -> OrderTicketBuilder {
        OrderTicketBuilder::new(order_type, order_side).price(price)
    }
}
