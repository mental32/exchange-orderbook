use axum::extract::Json;
use axum::response::IntoResponse;
use chrono::Utc;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SystemStatusResponse {
    pub status: String,
    pub timestamp: String,
}

pub async fn f() -> impl IntoResponse {
    let now = Utc::now();
    let timestamp = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    Json(SystemStatusResponse {
        status: "online".to_string(),
        timestamp,
    })
}
