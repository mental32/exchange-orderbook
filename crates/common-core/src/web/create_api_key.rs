use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;

use super::{middleware::auth::UserUuid, InternalApiState};

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyCreate {
    api_key_name: String,
    scopes: Vec<String>,
    ip_allowlist: Vec<String>,
}

pub async fn api_key_create(
    State(s): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Json(ApiKeyCreate {
        api_key_name,
        scopes,
        ip_allowlist,
    }): Json<ApiKeyCreate>,
) -> Response {
    sqlx::query!(
        "INSERT INTO api_keys (name, scopes, ip_restrictions) VALUES ($1, $2, $3)",
        api_key_name,
        &*scopes,
        &*ip_allowlist
    )
    .execute(&s.db())
    .await
    .unwrap();

    todo!()
}
