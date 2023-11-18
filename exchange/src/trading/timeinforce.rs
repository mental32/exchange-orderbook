//! Time in force options for orders.

use serde::{Deserialize, Serialize};

/// Time in force options for orders.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good Til Canceled, default. The order will remain open until it is either filled or canceled.
    #[serde(rename = "gtc")]
    GoodTilCanceled,
    /// Good Til Date specified. The order will remain open until it is either filled or canceled. it will automatically cancel at the specified timestamp.
    #[serde(rename = "gtd")]
    GoodTilDate,
    /// Immediate Or Cancel. The order must be filled immediately and any unfilled portion of the order will be canceled.
    #[serde(rename = "ioc")]
    ImmediateOrCancel,
    /// Fill Or Kill. The order must be filled immediately in its entirety or it will be canceled. The difference between this and IOC is that GTC orders will be placed on the book until canceled instead of being canceled at the end of the trading day.
    #[serde(rename = "fok")]
    FillOrKill,
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GoodTilCanceled
    }
}
