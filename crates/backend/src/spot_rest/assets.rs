use ap_actor::order_management::OrderManagement;
use axum::extract::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetInfo {
    pub aclass: String,
    pub altname: String,
    pub decimals: u8,
    pub display_decimals: u8,
}

pub async fn f(State(engine): State<OrderManagement>) -> impl IntoResponse {
    let assets = engine.get_enabled_assets();

    let mut result = BTreeMap::new();
    for asset in assets {
        result.insert(
            asset.clone(),
            AssetInfo {
                aclass: "currency".to_string(),
                altname: asset.clone(),
                decimals: 8,         // Standard for cryptocurrencies
                display_decimals: 4, // Standard display precision
            },
        );
    }

    Json(result)
}
