use std::ops::Deref;
use std::str::FromStr;

use argon2::password_hash::PasswordHashString;
use argon2::{Argon2, PasswordVerifier};
use axum::extract::{ConnectInfo, State};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{AppendHeaders, IntoResponse, IntoResponseParts, Response};
use axum::{Form, Json};
use axum_extra::extract::cookie::{self, Cookie};
use axum_extra::extract::CookieJar;
use axum_htmx::HxRequest;
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;

use crate::app_cx::VerifyLoginDetailsError;
use crate::password::{de_password_from_str, Password};
use crate::web::middleware::ip_address::rightmost_ip_address;

use super::InternalApiState;

#[derive(Debug, Clone, Deserialize)]
pub struct SessionCreate {
    email: EmailAddress,
    #[serde(deserialize_with = "de_password_from_str")]
    password: Password,
}

pub async fn f(
    State(state): State<InternalApiState>,
    jar: CookieJar,
    headers: HeaderMap,
    ConnectInfo(connect_info): ConnectInfo<std::net::SocketAddr>,
    HxRequest(hx): HxRequest,
    Form(body): Form<SessionCreate>,
) -> Response {
    use VerifyLoginDetailsError as V;
    tracing::trace!(?body, "session_create");

    let ip_address = rightmost_ip_address(&headers).unwrap_or(connect_info.ip());
    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|hv| hv.to_str().ok())
        .map(|st| st.to_owned());

    let user_uuid = match state
        .verify_login_details(&body.email, &body.password)
        .await
    {
        Ok(user_uuid) => user_uuid,
        Err(V::Unauthorized) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(V::Other(_)) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let session_token = match state
        .create_session(user_uuid, Some(ip_address), user_agent)
        .await
    {
        Ok(st) => st,
        Err(err) => {
            tracing::error!(?err, "could not create session");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    tracing::info!(?session_token, "session created");

    let session_token_cookie = Cookie::build(("session-token", session_token.as_str()))
        .max_age(time::Duration::hours(1))
        .path("/")
        .to_string();

    (
        AppendHeaders([
            (SET_COOKIE, session_token_cookie),
            (HeaderName::from_static("hx-redirect"), "/c".to_string()),
        ]),
        StatusCode::CREATED,
    )
        .into_response()
}
