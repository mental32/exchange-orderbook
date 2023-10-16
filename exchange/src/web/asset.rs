use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum InternalAssetKey {
    Static(&'static str),
    ByValue(Asset),
}

impl From<Asset> for InternalAssetKey {
    fn from(asset: Asset) -> Self {
        InternalAssetKey::ByValue(asset)
    }
}

impl From<&'static str> for InternalAssetKey {
    fn from(asset: &'static str) -> Self {
        InternalAssetKey::Static(asset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Asset {
    Bitcoin,
    Ether,
}
