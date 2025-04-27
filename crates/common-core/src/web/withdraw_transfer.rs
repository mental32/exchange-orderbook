use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};

use super::middleware::auth::UserUuid;
use super::InternalApiState;

pub struct WithdrawTransfer {
    currency: String,
    address: String,
    amount: String,
    max_fee: Option<String>,
}

pub async fn withdraw_transfer(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Json(body): Json<WithdrawTransfer>,
) -> Response {
    let db = state.db();

    let address_text = match sqlx::query!(
        "SELECT address_text FROM user_addresses WHERE user_id = $1 AND kind = 'withdrawal' AND currency = $2 AND address_text = $3",
        user_id,
        body.currency,
        body.address,
    )
    .fetch_optional(&db)
    .await
    {
        Ok(Some(rec)) => rec.address_text,
        Ok(None) => {
            tracing::trace!("user does not have matching requested withdrawal address registered");
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(err) => {
            tracing::error!(?err);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // verify user has necessary amount for transfer
    let user_amount = match sqlx::query!("").fetch_one(&db).await {
        Ok(_) => todo!(),
        Err(_) => todo!(),
    };

    // check if max_fee applies

    let tx = match db.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            tracing::error!(?err);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // transfer checks have completed, issue a transfer and write it to DB

    if let Err(err) = tx.commit().await {
        tracing::error!(?err);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    todo!()
}
