use serde::{Deserialize, Serialize};

use super::{OrderNotFound, asset::Assets, order_uuid::OrderUuid};

/// Data for canceling an order.
#[derive(Debug, Deserialize, Serialize)]
pub struct CancelOrder {
    /// the user that placed the order
    user_uuid: uuid::Uuid,
    /// the order to cancel
    order_uuid: OrderUuid,
}

impl CancelOrder {
    /// create a new [`CancelOrder``]
    pub fn new(user_uuid: uuid::Uuid, order_uuid: OrderUuid) -> Self {
        Self {
            user_uuid,
            order_uuid,
        }
    }
}

/// cancel an order
pub fn do_cancel_order(
    assets: &mut Assets,
    CancelOrder {
        user_uuid,
        order_uuid,
    }: CancelOrder,
) -> Result<(), OrderNotFound> {
    let (order_index, asset) = match assets.order_uuids.get(&order_uuid).cloned() {
        Some((a, b)) => (a, b),
        None => {
            return Err(OrderNotFound(user_uuid, order_uuid));
        }
    };

    let asset_book = assets.match_asset_mut(asset);

    asset_book
        .orderbook_mut()
        .remove(order_index)
        .expect("checked order");

    Ok(())
}
