use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Extension;

use super::middleware::auth::UserUuid;
use super::InternalApiState;

pub async fn f(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Path(tx_id): Path<String>,
) -> Response {
    todo!()
}
