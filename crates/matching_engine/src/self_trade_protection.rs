//! Self-trade protection of an order.

use serde::Deserialize;
use serde::Serialize;

use crate::pending_fill::PendingFill;

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

/// Apply self-trade protection to a pending fill.
pub fn self_trade_protection(pending_fill: &PendingFill<'_>, stp: SelfTradeProtection) {
    todo!("implement self trade protection: {stp:?}");
}
