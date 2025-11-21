use crate::middleware::clerk::Clerk;
use ap_actor::proc_router::ProcRouter;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GetWebSocketsToken {
    pub nonce: u64,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GetWebSocketsTokenResponse {
    pub token: String,
    pub expires: u64,
}

pub async fn f(
    State(_engine): State<ProcRouter>,
    Extension(_clerk): Extension<Clerk>,
    Json(_request): Json<GetWebSocketsToken>,
) -> Result<Json<GetWebSocketsTokenResponse>, (StatusCode, &'static str)> {
    Err((StatusCode::NOT_IMPLEMENTED, "endpoint not yet implemented"))
}
