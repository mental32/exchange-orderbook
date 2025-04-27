use std::any::Any;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{Extension, Form, Json};
use axum_htmx::HX_TRIGGER;
use serde::Deserialize;

use crate::bitcoin::proto::GetNewAddressRequest;
use crate::Asset;

use super::middleware::auth::UserUuid;
use super::InternalApiState;

#[derive(Debug, thiserror::Error)]
pub enum DeleteWithdrawalAddressError {
    #[error("The specified asset was invalid")]
    InvalidAsset,
    #[error("A withdrawal address for the specified asset already exists")]
    AddressNotFound,
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for DeleteWithdrawalAddressError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidAsset => {
                (StatusCode::BAD_REQUEST, "Invalid asset specified").into_response()
            }
            Self::Sqlx(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
            Self::AddressNotFound => (
                StatusCode::NOT_FOUND,
                "An address for this asset was not found",
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeletewithdrawalAddressParams {
    asset: String,
    address_text: String,
}

pub async fn f(
    State(mut state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Form(params): Form<DeletewithdrawalAddressParams>,
) -> Result<Response, DeleteWithdrawalAddressError> {
    let db = state.db();

    let asset = match params.asset.as_str() {
        "btc" | "BTC" => Asset::Bitcoin,
        "eth" | "ETH" => Asset::Ether,
        _ => {
            tracing::warn!(?params.asset, "invalid asset");
            return Err(DeleteWithdrawalAddressError::InvalidAsset);
        }
    };

    for (text, asset) in state.list_withdrawal_addrs(user_id).await? {
        if text == params.address_text && asset == params.asset {
            let rec = sqlx::query!(
                r#"
                DELETE FROM user_addresses
                WHERE user_id = $1 AND address_text = $2
                "#,
                user_id,
                params.address_text
            )
            .execute(&state.db())
            .await?;

            return Ok(([(HX_TRIGGER, "updateWithdrawalAddrs")],).into_response());
        }
    }

    Err(DeleteWithdrawalAddressError::AddressNotFound)
}
