use serde::{Deserialize, Serialize};

/// useful as a key in a map-like structure for when there are multiple ways to key an asset
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetKey {
    Static(&'static str),
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
    Bitcoin,
    Ether,
}

pub(crate) fn internal_asset_list() -> impl Iterator<Item = (AssetKey, Asset)> {
    use Asset as A;
    use AssetKey as K;

    [
        (K::from(A::Bitcoin), A::Bitcoin),
        (K::from("btc"), A::Bitcoin),
        (K::from("BTC"), A::Bitcoin),
        (K::from(A::Ether), A::Ether),
        (K::from("eth"), A::Ether),
        (K::from("ETH"), A::Ether),
    ]
    .into_iter()
}
