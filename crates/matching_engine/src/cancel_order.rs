//! logic for canceling orders in the orderbook
//!

use common_core::web::middleware::clerk::ClerkUserId;

use crate::orderbook::Order;
use crate::orderbook::OrderIndex;

use super::order_uuid::OrderUuid;

/// Data for canceling an order.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelOrder {
    /// the user that placed the order
    pub user_id: ClerkUserId,
    /// the order to cancel
    pub order_uuid: OrderUuid,
    /// the index of the order in the orderbook
    pub order_index: OrderIndex,
}

/// Cancel an order in the orderbook, returning the canceled order if it was found.
pub fn do_cancel_order(
    orderbook: &mut crate::orderbook::Orderbook,
    co: CancelOrder,
) -> Option<Order> {
    orderbook.remove(co.order_index)
}
