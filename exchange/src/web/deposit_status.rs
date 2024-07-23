use axum::{extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}, Extension};

use super::{middleware::auth::UserUuid, InternalApiState};

pub async fn f(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Path(tx_id): Path<String>
) -> Response {
    todo!()
}
