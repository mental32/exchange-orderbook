use std::str::FromStr;

use axum::extract::State;
use axum::headers::authorization::Bearer;
use axum::headers::{Authorization, Cookie, HeaderMapExt};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;

use axum::response::IntoResponse;
use redis::AsyncCommands;

use crate::web::InternalApiState;

/// A verified user ID from a session token
#[derive(Debug, Clone)]
pub struct UserUuid(pub uuid::Uuid);

/// Check if the request has a session-token cookie which is a
/// 32-byte hex string that should be a key in Redis with the format
/// session:{session-token} and the value should be the user ID (UUID)
pub async fn validate_session_token_redis<B>(
    State(state): State<InternalApiState>,
    mut request: Request<B>,
    next: Next<B>,
) -> axum::response::Response {
    // Attempt to get a Redis connection from the pool
    let mut conn = match state.redis.get_async_connection().await {
        Ok(conn) => conn,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    };

    // Extract the session-token cookie from the request headers
    let session_token = match request.headers().typed_get::<Authorization<Bearer>>() {
        Some(auth) => auth.0.token().to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: session-token cookie missing",
            )
                .into_response();
        }
    };

    // Query Redis to validate the session token
    let redis_key = format!("session:{}", session_token);
    let user_id = conn
        .get(&redis_key)
        .await
        .unwrap_or(None)
        .map(|st: String| uuid::Uuid::from_str(&st));

    match user_id {
        Some(Ok(uuid)) => {
            // Session token is valid; proceed to the next middleware or handler
            request.extensions_mut().insert(UserUuid(uuid));
            next.run(request).await
        }
        None | Some(Err(_)) => {
            // Session token is invalid; return an unauthorized error
            (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: invalid session token",
            )
                .into_response()
        }
    }
}
