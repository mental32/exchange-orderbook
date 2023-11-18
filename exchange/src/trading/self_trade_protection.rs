//! Self-trade protection of an order.

use serde::{Deserialize, Serialize};

/// The self-trade protection of an order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelfTradeProtection {
    /// Decrease and cancel.
    #[serde(rename = "dc")]
    DecreaseCancel,
    /// Cancel oldest.
    #[serde(rename = "co")]
    CancelOldest,
    /// Cancel newest.
    #[serde(rename = "cn")]
    CancelNewest,
    /// Cancel both.
    #[serde(rename = "cb")]
    CancelBoth,
}

impl Default for SelfTradeProtection {
    fn default() -> Self {
        Self::DecreaseCancel
    }
}
