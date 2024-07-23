use std::ops::Deref;
use std::str::FromStr;

use argon2::password_hash::PasswordHashString;
use argon2::{Argon2, PasswordVerifier};
use axum::extract::State;
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{AppendHeaders, IntoResponse, IntoResponseParts, Response};
use axum::Json;
use axum_extra::extract::cookie::{self, Cookie};
use axum_extra::extract::CookieJar;
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;

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

pub async fn f(
    State(state): State<InternalApiState>,
    jar: CookieJar,
    Json(body): Json<SessionCreate>,
) -> Response {
    tracing::trace!(?body, "session_create");

    let db = state.db();

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

    tracing::info!(user_id = ?rec.id, "user found");

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

    // generate a session token and store it
    let session_token = {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rand::Rng::fill(&mut rng, &mut bytes[..]);
        hex::encode(bytes)
    };

    if let Err(err) = sqlx::query!(
        "INSERT INTO session_tokens (token, max_age, user_id) VALUES ($1, $2, $3);",
        session_token.as_bytes(),
        3600,
        rec.id,
    )
    .execute(&state.db())
    .await
    {
        tracing::error!(?err, "session_token insert failure");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    tracing::info!(?session_token, "session created");

    let session_token_cookie = Cookie::build(("session-token", session_token.as_str()))
        .max_age(time::Duration::hours(1))
        .to_string();

    (
        AppendHeaders([(SET_COOKIE, session_token_cookie)]),
        StatusCode::CREATED,
    )
        .into_response()
}
