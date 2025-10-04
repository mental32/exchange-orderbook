//! "ISO 4217"-like codes for assets, e.g. "USD", "BTC", "ETH", "USDT" but up to 8 characters long

use std::fmt::Display;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolVocabulary(Arc<[Box<str>]>);

impl FromIterator<String> for SymbolVocabulary {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        SymbolVocabulary(
            iter.into_iter()
                .map(|s: String| s.into_boxed_str())
                .collect(),
        )
    }
}

/// Asset codes represent an asset in the exchange like "USD", "BTC", "ETH", "USDT"
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetCode(u16);

impl AssetCode {
    /// Create an AssetCode from a string slice if it is valid
    /// Valid asset codes are between 3 and 8 characters long and contain only uppercase letters
    /// Returns None if the asset code is invalid
    pub fn from_str_and_vocabulary(
        st: &str,
        SymbolVocabulary(slice): &SymbolVocabulary,
    ) -> Option<Self> {
        let code = slice.iter().position(|s: &Box<str>| s.as_ref().eq(st))?;
        Some(AssetCode(code as u16))
    }

    #[track_caller]
    pub fn as_str<'a>(&self, SymbolVocabulary(slice): &'a SymbolVocabulary) -> &'a str {
        slice
            .get(self.0 as usize)
            .as_ref()
            .expect("bug: there might be multiple symbol vocabularies?")
    }
}
