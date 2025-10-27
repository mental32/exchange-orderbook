use axum::extract::Json;
use axum::response::IntoResponse;
use chrono::Utc;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeResponse {
    pub unixtime: u64,
    pub rfc1123: String,
}

pub async fn f() -> impl IntoResponse {
    let now = Utc::now();
    let unixtime = now.timestamp() as u64;
    let rfc1123 = now.to_rfc2822();

    Json(TimeResponse { unixtime, rfc1123 })
}
