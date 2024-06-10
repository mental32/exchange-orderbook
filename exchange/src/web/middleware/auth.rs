use std::str::FromStr;

use axum::body::Body;
use axum::extract::State;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::{Authorization, HeaderMapExt};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;

use axum::response::IntoResponse;
use axum_extra::extract::CookieJar;
use chrono::{Datelike, Timelike};
use sqlx::types::time::{Date, PrimitiveDateTime, Time};

use crate::web::InternalApiState;

/// A verified user ID from a session token
#[derive(Debug, Clone)]
pub struct UserUuid(pub uuid::Uuid);

/// Enforce that the request has a session-token cookie
///
/// * A session-token cookie is a randomly generated 32-byte hex-encoded string.
/// * session-token cookies can expire which is checked here.
///
/// If the checks pass a [`UserUuid`] extension will be added to the request
/// which specifies the user id of the requester.
///
pub async fn validate_session_token(
    State(state): State<InternalApiState>,
    // jar: CookieJar,
    mut request: Request<Body>,
    next: Next,
) -> axum::response::Response {
    // Extract the session-token cookie from the request headers
    let jar = CookieJar::from_headers(request.headers());
    let session_token = if let Some(t) = jar.get("session-token") {
        t.value_trimmed()
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: session-token cookie missing",
        )
            .into_response();
    };

    let rec = match sqlx::query!(
        "SELECT * FROM session_tokens WHERE token = $1",
        session_token.as_bytes()
    )
    .fetch_optional(&state.app_cx.db_pool)
    .await
    {
        Ok(r) => r,
        Err(err) => {
            tracing::error!(?err, "session-token select failure");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Try again later").into_response();
        }
    };

    match rec {
        Some(rec) => {
            let now = chrono::Utc::now();
            let now_d = Date::from_ordinal_date(now.year(), now.ordinal() as _).unwrap();
            let now_t =
                Time::from_hms(now.hour() as _, now.minute() as _, now.second() as _).unwrap();

            if (now_d > rec.expires_at.date()) && (now_t >= rec.expires_at.time()) {
                return (
                    StatusCode::UNAUTHORIZED,
                    "Unauthorized: session-token has expired",
                )
                    .into_response();
            }

            // Session token is valid; proceed to the next middleware or handler
            request.extensions_mut().insert(UserUuid(rec.user_id));
            next.run(request).await
        }
        None => {
            // Session token is invalid; return an unauthorized error
            (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: invalid session token",
            )
                .into_response()
        }
    }
}
