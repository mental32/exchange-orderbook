use axum::response::IntoResponse;

pub async fn f() -> impl IntoResponse {
    let now = chrono::Utc::now();

    now.to_rfc3339()
}
