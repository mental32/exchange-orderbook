//! [`AssetPairRow`] from the database and related types like [`BaseQuote`]
//!

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::decimal::Decimal;
use sqlx::types::time::PrimitiveDateTime;

/// type used in `created_at` and `updated_at` fields in [`AssetPairRow`], should be time::[`PrimitiveDateTime`]
pub type DateTime = PrimitiveDateTime;

/// Asset pair represented as (base, quote)
pub type BaseQuote = (AssetCode, AssetCode);

/// See `t_trading_asset_pairs` table in the DB schema
#[derive(Debug, Clone)]
#[allow(missing_docs)] // allow missing docs for struct fields that map to DB columns
pub struct AssetPairRow {
    pub id: i32,
    pub base_asset: String,
    pub quote_asset: String,
    pub status: String,
    pub min_order_size: Decimal,
    pub max_order_size: Option<Decimal>,
    pub price_tick_size: Decimal,
    pub quantity_tick_size: Decimal,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl AssetPairRow {
    /// the asset pair as (base, quote)
    pub fn base_quote(
        &self,
        symbol_vocabulary: &SymbolVocabulary,
    ) -> Option<(AssetCode, AssetCode)> {
        let base = AssetCode::from_str_and_vocabulary(&self.base_asset, symbol_vocabulary);
        let quote = AssetCode::from_str_and_vocabulary(&self.quote_asset, symbol_vocabulary);
        base.zip(quote)
    }
}
