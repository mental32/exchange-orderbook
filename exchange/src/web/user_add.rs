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
pub struct UserAdd {
    name: String,
    email: EmailAddress,
    password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UserAddError {
    #[error("password hash error")]
    PasswordHashError,
    #[error("email has already been used")]
    EmailAlreadyUsed,
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for UserAddError {
    fn into_response(self) -> axum::response::Response {
        match self {
            UserAddError::PasswordHashError => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            UserAddError::EmailAlreadyUsed => {
                (StatusCode::CONFLICT, "email has already been used").into_response()
            }
            UserAddError::Sqlx(err) => {
                tracing::error!(?err, "sqlx error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn user_add(
    State(state): State<InternalApiState>,
    Json(body): Json<UserAdd>,
) -> Result<Json<serde_json::Value>, UserAddError> {
    let password_hash = tokio::task::spawn_blocking({
        let password = body.password.clone();

        move || {
            let argon2 = Argon2::default();
            let salt = SaltString::generate(&mut OsRng);

            let password_hash =
                argon2
                    .hash_password(password.as_bytes(), &salt)
                    .map_err(|err| {
                        tracing::error!(?err, "failed to hash password");
                        UserAddError::PasswordHashError
                    })?;

            Ok::<_, UserAddError>(password_hash.serialize())
        }
    })
    .await
    .map_err(|_| UserAddError::PasswordHashError)??;

    // check if the email has already been used
    let rec = sqlx::query!(
        r#"
        SELECT id FROM users WHERE email = $1
        "#,
        body.email.as_str()
    )
    .fetch_optional(&state.db_pool)
    .await?;

    if rec.is_some() {
        return Err(UserAddError::EmailAlreadyUsed);
    }

    sqlx::query!(
        r#"
        INSERT INTO users (name, email, password_hash)
        VALUES (
                $1,
                $2,
                $3
            );
        "#,
        body.name,
        body.email.as_str(),
        password_hash.as_bytes(),
    )
    .execute(&state.db_pool)
    .await?;

    let rec = sqlx::query!("SELECT id FROM users WHERE email = $1", body.email.as_str())
        .fetch_one(&state.db_pool)
        .await?;

    Ok(Json(serde_json::json!({
        "user_id": rec.id.to_string(),
    })))
}
