use ap_actor::proc_router::ProcRouter;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct TradesRequest {
    pub pair: String,
    #[serde(default)]
    pub since: Option<u64>,
    #[serde(default)]
    pub count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct TradesResponse(Vec<(String, String, u64, String, String, String, u64)>);

/// Get recent trades for one or more asset pairs
pub async fn f(
    State(_engine): State<ProcRouter>,
    Json(_payload): Json<TradesRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    Err::<Json<()>, _>((
        StatusCode::SERVICE_UNAVAILABLE,
        "Market data not implemented",
    ))
}
