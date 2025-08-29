//! "ISO 4217"-like codes for assets, e.g. "USD", "BTC", "ETH", "USDT" but up to 8 characters long

// XXX: maybe its better to have AssetCode newtype an integer, and then have some kind of vocabulary table mapping integers to real assets?

/// Asset codes represent an asset in the exchange like "USD", "BTC", "ETH", "USDT"
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetCode(pub(crate) [u8; 8]);

impl AssetCode {
    /// Create an AssetCode from a string slice if it is valid
    /// Valid asset codes are between 3 and 8 characters long and contain only uppercase letters
    /// Returns None if the asset code is invalid
    pub fn from_str(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() >= 3 && bytes.len() <= 8 {
            return None;
        }
        let mut code = [0u8; 8];
        code[..bytes.len()].copy_from_slice(bytes);
        Some(AssetCode(code))
    }
}

impl std::fmt::Display for AssetCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = std::str::from_utf8(&self.0).map_err(|_| std::fmt::Error)?;
        write!(f, "{s}")
    }
}
