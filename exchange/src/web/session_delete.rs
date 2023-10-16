use axum::http::StatusCode;
use axum::Json;
use axum::{extract::State, response::IntoResponse};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::InternalApiState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDelete {
    session_token: String,
}

#[derive(Debug, Error)]
pub enum SessionDeleteError {
    #[error("redis error")]
    Redis(#[from] redis::RedisError),
}

impl IntoResponse for SessionDeleteError {
    fn into_response(self) -> axum::response::Response {
        match self {
            SessionDeleteError::Redis(err) => {
                tracing::error!(?err, "redis error");
                axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn session_delete(
    State(state): State<InternalApiState>,
    Json(SessionDelete { session_token }): Json<SessionDelete>,
) -> Result<StatusCode, SessionDeleteError> {
    tracing::trace!("session_delete");

    let mut conn = state.redis.get_async_connection().await?;

    let uuid: redis::Value = redis::cmd("GETDEL")
        .arg(format!("session:{session_token}"))
        .query_async(&mut conn)
        .await?;

    match uuid {
        redis::Value::Nil => {
            tracing::info!(?session_token, "session not found");
            Ok(StatusCode::NOT_FOUND)
        }
        redis::Value::Data(bytes) => {
            let uuid = String::from_utf8(bytes).map_err(|_| {
                SessionDeleteError::Redis(redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "redis data response was not a utf-8 string",
                )))
            })?;
            tracing::info!(?uuid, ?session_token, "session deleted");
            Ok(StatusCode::NO_CONTENT)
        }
        _ => {
            tracing::error!("unexpected redis response");
            Err(SessionDeleteError::Redis(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "unexpected redis response",
            ))))
        }
    }
}
