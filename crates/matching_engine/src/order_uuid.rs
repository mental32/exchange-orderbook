//! type-safe order UUIDs.

/// newtype for order UUIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderUuid(pub uuid::Uuid);

impl From<uuid::Uuid> for OrderUuid {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
    }
}
