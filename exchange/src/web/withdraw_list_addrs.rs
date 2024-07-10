use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse as _, Response},
    Extension, Json,
};

use super::{middleware::auth::UserUuid, InternalApiState};

pub async fn withdraw_list_addrs(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
) -> Response {
    let db = state.db();
    let v_rec = match sqlx::query!(
        "SELECT address_text, currency
    FROM user_addresses
    WHERE user_id = $1
      AND kind = 'withdrawal';
    ",
        user_id
    )
    .fetch_all(&db)
    .await
    {
        Ok(v_rec) => v_rec,
        Err(err) => {
            tracing::error!(?err, "selecting deposit addresses for user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Json(serde_json::json!(v_rec
        .into_iter()
        .map(|rec| serde_json::json!({"address": rec.address_text, "currency": rec.currency}))
        .collect::<Vec<_>>()))
    .into_response()
}
