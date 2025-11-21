use matching_engine::asset_pair::BaseQuote;
use matching_engine::order_uuid::OrderUuid;
use matching_engine::orderbook::OrderIndex;

use crate::proc_router::CancelOrderBy;

#[derive(Debug, Clone)]
pub struct OpenOrder {
    pub base_quote: BaseQuote,
    pub order_uuid: OrderUuid,
    pub order_index: OrderIndex,
    pub userref: Option<u32>,
    pub cl_ord_id: Option<String>,
}

#[derive(Debug)]
pub struct UserProfile {
    pub open_orders: Vec<OpenOrder>,
}

impl UserProfile {
    pub fn isolate_orders_for_cancel(&self, cancel_order_by: &CancelOrderBy) -> Vec<OpenOrder> {
        match cancel_order_by {
            CancelOrderBy::TxId(order_uuid) => self
                .open_orders
                .iter()
                .find(|o| o.order_uuid == *order_uuid)
                .map(|o| vec![o.clone()])
                .unwrap_or_default(),
            CancelOrderBy::Userref(target) => self
                .open_orders
                .iter()
                .filter(|p| p.userref == Some(*target))
                .cloned()
                .collect(),
            CancelOrderBy::ClientOrderId(cl_ord_id) => self
                .open_orders
                .iter()
                .find(|p| {
                    p.cl_ord_id
                        .as_ref()
                        .map(|st| st == cl_ord_id)
                        .unwrap_or(false)
                })
                .map(|o| vec![o.clone()])
                .unwrap_or_default(),
        }
    }
}
