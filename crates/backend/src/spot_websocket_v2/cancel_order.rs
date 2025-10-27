#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelOrder {
    /// A list of `order_id` identifiers.
    order_id: Vec<String>,
    /// A list of client `cl_ord_id` identifiers.
    cl_ord_id: Vec<String>,
    /// A list of client `order_userref` identifiers.
    order_userref: Vec<String>,
}
