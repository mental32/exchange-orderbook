use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde::Serialize;

use ap_actor::order_management::OrderManagement;

#[derive(Debug, Deserialize)]
pub struct SpreadRequest {
    pub pair: String,
    #[serde(default)]
    pub since: Option<u64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct SpreadEntry(Vec<(String, String, String, String, u64)>);

/// Get recent spread data for one or more asset pairs
pub async fn f(
    State(_engine): State<OrderManagement>,
    Json(_payload): Json<SpreadRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    Err::<Json<()>, _>((
        StatusCode::SERVICE_UNAVAILABLE,
        "Market data not implemented",
    ))
}
