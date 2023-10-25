use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeInForce {
    #[serde(rename = "gtc")]
    GoodTilCanceled,
    #[serde(rename = "gtd")]
    GoodTilDate,
    #[serde(rename = "ioc")]
    ImmediateOrCancel,
    #[serde(rename = "fok")]
    FillOrKill,
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GoodTilCanceled
    }
}
