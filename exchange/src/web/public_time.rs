use axum::response::IntoResponse;

pub async fn public_time() -> impl IntoResponse {
    let now = chrono::Utc::now();

    now.to_rfc3339()
}
