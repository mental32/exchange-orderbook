use crate::app_cx::CreateUserError;
use crate::password::{de_password_from_str, Password};
use crate::web::middleware::ip_address::rightmost_ip_address;

use super::InternalApiState;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;

use axum::extract::{ConnectInfo, State};
use axum::http::header::{SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::{Form, Json};

use axum_extra::extract::cookie::Cookie;
use axum_htmx::{HxRequest, HX_REDIRECT};
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
            CreateUserError::Sqlx(err) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn f(
    State(state): State<InternalApiState>,
    HxRequest(hx): HxRequest,
    ConnectInfo(connect_info): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Form(body): Form<UserCreate>,
) -> Result<Response, CreateUserError> {
    let password_hash =
        tokio::task::spawn_blocking({ move || body.password.argon2_hash_password() })
            .await
            .map_err(|_| CreateUserError::PasswordHashError)?
            .map_err(|_| CreateUserError::PasswordHashError)?; // TODO: use a more specific error on one of these branches

    let user_uuid = state
        .create_user(body.name.as_str(), body.email.as_str(), password_hash)
        .await?;

    let ip_address = rightmost_ip_address(&headers).unwrap_or(connect_info.ip());
    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|hv| hv.to_str().ok())
        .map(|st| st.to_owned());

    let session_token = match state
        .create_session(user_uuid, Some(ip_address), user_agent)
        .await
    {
        Ok(st) => st,
        Err(err) => {
            tracing::error!(?err, "could not create session");
            return Err(CreateUserError::Sqlx(err));
        }
    };

    tracing::info!(?session_token, "session created");

    let session_token_cookie = Cookie::build(("session-token", session_token.as_str()))
        .max_age(time::Duration::hours(1))
        .path("/")
        .to_string();

    let user_uuid = Json(serde_json::json!({
        "user_id": user_uuid.to_string(),
    }));

    let resp = if hx {
        (
            AppendHeaders([
                (SET_COOKIE, session_token_cookie),
                (HX_REDIRECT, "/".to_owned()),
            ]),
            user_uuid,
        )
            .into_response()
    } else {
        user_uuid.into_response()
    };

    Ok(resp)
}
