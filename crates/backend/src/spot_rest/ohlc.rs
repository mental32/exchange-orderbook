use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde::Serialize;

use ap_actor::proc_router::ProcRouter;

#[derive(Debug, Deserialize)]
pub struct OhlcRequest {
    pub pair: String,
    #[serde(default)]
    pub interval: Option<u64>,
    #[serde(default)]
    pub since: Option<u64>,
}

#[derive(Debug, Serialize)]
struct OhlcResponse(Vec<(u64, String, String, String, String, String, String, u64)>);

/// Get OHLC (Open, High, Low, Close) data for a given asset pair
pub async fn f(
    State(_engine): State<ProcRouter>,
    Json(_payload): Json<OhlcRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    Err::<Json<()>, _>((
        StatusCode::SERVICE_UNAVAILABLE,
        "Market data not implemented",
    ))
}
