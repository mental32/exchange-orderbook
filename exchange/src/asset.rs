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

pub(crate) fn internal_asset_list() -> impl Iterator<Item = (InternalAssetKey, Asset)> {
    use Asset as A;
    use InternalAssetKey as K;

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
