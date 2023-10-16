use super::InternalApiState;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserDelete {
    id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum UserDeleteError {
    #[error("user not found")]
    UserNotFound,
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for UserDeleteError {
    fn into_response(self) -> axum::response::Response {
        match self {
            UserDeleteError::UserNotFound => {
                (StatusCode::NOT_FOUND, "user not found").into_response()
            }
            UserDeleteError::Sqlx(err) => {
                tracing::error!(?err, "sqlx error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn user_delete(
    State(state): State<InternalApiState>,
    Json(body): Json<UserDelete>,
) -> Result<Json<serde_json::Value>, UserDeleteError> {
    let updated_rows = sqlx::query!(
        r#"
        UPDATE users SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL
        "#,
        body.id
    )
    .execute(&state.db_pool)
    .await?;

    if updated_rows.rows_affected() == 0 {
        return Err(UserDeleteError::UserNotFound);
    }

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}
