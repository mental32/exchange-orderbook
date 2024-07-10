//! Asset types

use serde::{Deserialize, Serialize};

/// useful as a key in a map-like structure for when there are multiple ways to key an asset
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetKey {
    /// reference by a static string (e.g. "btc" or "eth")
    Static(&'static str),
    /// reference by the asset itself
    ByValue(Asset),
}

impl From<Asset> for AssetKey {
    fn from(asset: Asset) -> Self {
        AssetKey::ByValue(asset)
    }
}

impl From<&'static str> for AssetKey {
    fn from(asset: &'static str) -> Self {
        AssetKey::Static(asset)
    }
}

/// An asset that can be traded on the exchange
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Asset {
    /// Bitcoin
    Bitcoin,
    /// Ethereum
    Ether,
}


/// Helper for the asset list
pub trait ContainsAsset {
    /// check if an asset-key is present in the list
    fn contains_asset(&self, key: &AssetKey) -> bool;
}

impl ContainsAsset for [(AssetKey, Asset)] {
    fn contains_asset(&self, key: &AssetKey) -> bool {
        for (k, v) in self {
            if k == key {
                return true;
            }
        }
        return false;
    }
}

pub(crate) fn internal_asset_list() -> &'static [(AssetKey, Asset)] {
    use {Asset as A, AssetKey as K};

    [
        (K::ByValue(A::Bitcoin), A::Bitcoin),
        (K::Static("btc"), A::Bitcoin),
        (K::Static("BTC"), A::Bitcoin),
        (K::ByValue(A::Ether), A::Ether),
        (K::Static("eth"), A::Ether),
        (K::Static("ETH"), A::Ether),
    ].as_slice()
}
