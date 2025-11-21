use crate::middleware::clerk::Clerk;
use ap_actor::proc_router::ProcRouter;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelAll {
    pub nonce: u64,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelAllResponse {
    pub count: usize,
}

pub async fn f(
    State(_engine): State<ProcRouter>,
    Extension(_clerk): Extension<Clerk>,
    Json(_request): Json<CancelAll>,
) -> Result<Json<CancelAllResponse>, (StatusCode, &'static str)> {
    Err((StatusCode::NOT_IMPLEMENTED, "endpoint not yet implemented"))
}
