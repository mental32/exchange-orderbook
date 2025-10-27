use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

use ap_actor::order_management::OrderManagement;

#[derive(Debug, Serialize)]
struct TickerEntry {
    a: [String; 3],
    b: [String; 3],
    c: [String; 2],
    v: [String; 2],
    p: [String; 2],
    t: [u64; 2],
    l: [String; 2],
    h: [String; 2],
    o: String,
}

/// Get ticker information for one or more asset pairs
pub async fn f(
    State(_engine): State<OrderManagement>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    Err::<Json<()>, _>((
        StatusCode::SERVICE_UNAVAILABLE,
        "Market data not implemented",
    ))
}
