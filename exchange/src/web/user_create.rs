use crate::app_cx::CreateUserError;
use crate::password::{de_password_from_str, Password};

use super::InternalApiState;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};

use email_address::EmailAddress;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserCreate {
    name: String,
    email: EmailAddress,
    #[serde(deserialize_with = "de_password_from_str")]
    password: Password,
}

impl IntoResponse for CreateUserError {
    fn into_response(self) -> axum::response::Response {
        match self {
            CreateUserError::PasswordHashError => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            CreateUserError::EmailUniqueViolation(_) => {
                (StatusCode::CONFLICT, "email has already been used").into_response()
            }
            CreateUserError::GenericSqlxError(err) => {
                tracing::error!(?err, "sqlx error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn user_create(
    State(state): State<InternalApiState>,
    Json(body): Json<UserCreate>,
) -> Result<Json<serde_json::Value>, CreateUserError> {
    let password_hash =
        tokio::task::spawn_blocking({ move || body.password.argon2_hash_password() })
            .await
            .map_err(|_| CreateUserError::PasswordHashError)?
            .map_err(|_| CreateUserError::PasswordHashError)?;

    let user_id = state
        .create_user(body.name.as_str(), body.email.as_str(), password_hash)
        .await?;

    Ok(Json(serde_json::json!({
        "user_id": user_id.to_string(),
    })))
}
