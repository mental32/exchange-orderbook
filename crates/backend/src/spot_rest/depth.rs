use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde::Serialize;

use ap_actor::order_management::OrderManagement;

#[derive(Debug, Deserialize)]
pub struct DepthRequest {
    pub pair: String,
    #[serde(default)]
    pub count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct DepthLevels {
    bids: Vec<(String, String, u64)>,
    asks: Vec<(String, String, u64)>,
}

/// Get order book (depth) data for one or more asset pairs
pub async fn f(
    State(_engine): State<OrderManagement>,
    Json(_payload): Json<DepthRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    Err::<Json<()>, _>((
        StatusCode::SERVICE_UNAVAILABLE,
        "Market data not implemented",
    ))
}
