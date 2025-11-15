//! [`AssetPairRow`] from the database and related types like [`BaseQuote`]
//!

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::decimal::Decimal;
use sqlx::prelude::FromRow;
use sqlx::types::time::PrimitiveDateTime;

/// type used in `created_at` and `updated_at` fields in [`AssetPairRow`], should be time::[`PrimitiveDateTime`]
pub type DateTime = PrimitiveDateTime;

/// Asset pair represented as (base, quote)
pub type BaseQuote = (AssetCode, AssetCode);

/// See `t_trading_asset_pairs` table in the DB schema
#[derive(Debug, Clone, FromRow)]
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
    pub altname: String,
    pub wsname: String,
    pub pair_decimals: i32,
    pub cost_decimals: i32,
    pub lot_decimals: i32,
    pub lot: String,
    pub lot_multiplier: i32,
    pub aclass_base: String,
    pub aclass_quote: String,
    pub leverage_buy: Vec<i32>,
    pub leverage_sell: Vec<i32>,
    pub fees: serde_json::Value,
    pub fees_maker: serde_json::Value,
    pub fee_volume_currency: String,
    pub margin_call: i32,
    pub margin_stop: i32,
    pub costmin: Decimal,
    pub tick_size: Decimal,
    pub long_position_limit: Option<i32>,
    pub short_position_limit: Option<i32>,
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
