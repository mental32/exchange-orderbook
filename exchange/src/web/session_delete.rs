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
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for SessionDeleteError {
    fn into_response(self) -> axum::response::Response {
        match self {
            SessionDeleteError::Sqlx(err) => {
                tracing::error!(?err, "sqlx error");
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

    let rec = sqlx::query!(
        "
    WITH deleted_token AS (
        DELETE FROM session_tokens
        WHERE token = $1
        RETURNING *
    )
    SELECT * FROM deleted_token;
    ",
        session_token.as_bytes()
    )
    .fetch_optional(&state.app_cx.db_pool)
    .await?;

    match rec {
        None => {
            tracing::info!(?session_token, "session not found");
            Ok(StatusCode::NOT_FOUND)
        }
        Some(rec) => {
            tracing::info!(uuid = ?rec.user_id, ?session_token, "session deleted");
            Ok(StatusCode::NO_CONTENT)
        }
    }
}
