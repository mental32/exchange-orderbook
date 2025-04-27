use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};

use super::middleware::auth::UserUuid;
use super::InternalApiState;

pub async fn f(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
) -> Response {
    let v_rec = match state.list_deposit_addrs(user_id).await {
        Ok(v_rec) => v_rec,
        Err(err) => {
            tracing::error!(?err, "selecting deposit addresses for user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Json(serde_json::json!(v_rec
        .into_iter()
        .map(|(address_text, currency)| serde_json::json!({"address": address_text, "currency": currency}))
        .collect::<Vec<_>>()))
    .into_response()
}
