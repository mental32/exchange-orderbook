use ap_actor::order_management::OrderManagement;
use axum::extract::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetPairInfo {
    pub altname: String,
    pub wsname: String,
    pub aclass_base: String,
    pub aclass_quote: String,
    pub base: String,
    pub quote: String,
    pub pair_decimals: u8,
    pub lot_decimals: u8,
}

pub async fn f(State(engine): State<OrderManagement>) -> impl IntoResponse {
    let pairs = engine.get_enabled_pairs();

    let mut result = BTreeMap::new();
    for (base, quote) in pairs {
        let base_str = base.as_str().to_string();
        let quote_str = quote.as_str().to_string();
        let pair_name = format!("{}/{}", base_str, quote_str);

        result.insert(
            pair_name.clone(),
            AssetPairInfo {
                altname: format!("{}{}", base_str, quote_str),
                wsname: pair_name.clone(),
                aclass_base: "currency".to_string(),
                aclass_quote: "currency".to_string(),
                base: base_str,
                quote: quote_str,
                pair_decimals: 4,
                lot_decimals: 8,
            },
        );
    }

    Json(result)
}
