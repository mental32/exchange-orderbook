/// The unique identifier for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct OrderUuid(pub uuid::Uuid);
impl OrderUuid {
    pub fn new_v4() -> OrderUuid {
        OrderUuid(uuid::Uuid::new_v4())
    }
}
