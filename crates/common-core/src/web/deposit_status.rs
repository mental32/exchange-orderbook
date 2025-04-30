use axum::Extension;
use axum::extract::{Path, State};
use axum::response::Response;

use super::InternalApiState;
use super::middleware::auth::UserUuid;

pub async fn f(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Path(tx_id): Path<String>,
) -> Response {
    todo!()
}
