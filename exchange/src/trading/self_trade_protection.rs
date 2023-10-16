use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelfTradeProtection {
    #[serde(rename = "dc")]
    DecreaseCancel,
    #[serde(rename = "co")]
    CancelOldest,
    #[serde(rename = "cn")]
    CancelNewest,
    #[serde(rename = "cb")]
    CancelBoth,
}
