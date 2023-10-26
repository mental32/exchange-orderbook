use std::ops::Deref;
use std::str::FromStr;

use argon2::password_hash::PasswordHashString;
use argon2::{Argon2, PasswordVerifier};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use email_address::EmailAddress;
use serde::{Deserialize, Serialize};

use super::InternalApiState;

#[derive(Clone, Serialize, Deserialize)]
struct Password(String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Password").field(&"*").finish()
    }
}

impl Deref for Password {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreate {
    email: EmailAddress,
    password: Password,
}

pub async fn session_create(
    State(state): State<InternalApiState>,
    Json(body): Json<SessionCreate>,
) -> Response {
    tracing::trace!(?body, "session_create");

    let db = state.app_cx.db_pool.clone();

    let rec = match sqlx::query!(
        // language=PostgreSQL
        r#"
        SELECT id, password_hash FROM users
        WHERE email = $1
        "#,
        body.email.as_str()
    )
    .fetch_one(&db)
    .await
    {
        Ok(rec) => rec,
        Err(sqlx::Error::RowNotFound) => {
            tracing::info!(email = ?body.email, "user email not found");
            return StatusCode::UNAUTHORIZED.into_response();
        }
        Err(err) => {
            tracing::error!(?err, "failed to query user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    tracing::info!(?rec, "user found");

    if tokio::task::spawn_blocking({
        let from_utf8 = &String::from_utf8(rec.password_hash).unwrap();
        let phs = PasswordHashString::from_str(from_utf8.as_str()).unwrap();
        let password_as_bytes = body.password.as_bytes().to_owned();

        move || {
            Argon2::default()
                .verify_password(&password_as_bytes, &phs.password_hash())
                .is_err()
        }
    })
    .await
    .unwrap_or(false)
    {
        tracing::info!("password mismatch");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // generate a session token and store it in redis
    let session_token = {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rand::Rng::fill(&mut rng, &mut bytes[..]);
        hex::encode(bytes)
    };

    let session_token_key = format!("session:{}", session_token);

    let mut conn = state.redis.get_async_connection().await.unwrap();

    let () = redis::cmd("SET")
        .arg(session_token_key)
        .arg(rec.id.to_string())
        .arg("NX") // only set if it doesn't exist
        .arg("EX") // expire after
        .arg(24 * 60 * 60) // 24 hours
        .query_async(&mut conn)
        .await
        .unwrap();

    tracing::info!(?session_token, "session created");

    Json(serde_json::json!( {
        "session_token": session_token
    }))
    .into_response()
}
