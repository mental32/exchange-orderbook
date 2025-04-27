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
pub enum CreateWithdrawalAddressError {
    #[error("The specified asset was invalid")]
    InvalidAsset,
    #[error("A withdrawal address for the specified asset already exists")]
    AlreadyExists,
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for CreateWithdrawalAddressError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidAsset => {
                (StatusCode::BAD_REQUEST, "Invalid asset specified").into_response()
            }
            Self::Sqlx(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
            Self::AlreadyExists => (
                StatusCode::CONFLICT,
                "An address for this asset already exists",
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatewithdrawalAddressParams {
    asset: String,
    address_text: String,
}

pub async fn f(
    State(mut state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Form(params): Form<CreatewithdrawalAddressParams>,
) -> Result<Response, CreateWithdrawalAddressError> {
    let db = state.db();

    let asset = match params.asset.as_str() {
        "btc" | "BTC" => Asset::Bitcoin,
        "eth" | "ETH" => Asset::Ether,
        _ => {
            tracing::warn!(?params.asset, "invalid asset");
            return Err(CreateWithdrawalAddressError::InvalidAsset);
        }
    };

    let addrs = state.list_withdrawal_addrs(user_id).await?;
    if addrs
        .iter()
        .any(|(text, asset)| asset.as_str() == params.asset && text.as_str() == params.address_text)
    {
        return Err(CreateWithdrawalAddressError::AlreadyExists);
    }

    let rec = sqlx::query!(
        r#"
        INSERT INTO user_addresses (user_id, address_text, kind, currency)
        VALUES ($1, $2, 'withdrawal', $3)
        RETURNING id
        "#,
        user_id,
        params.address_text,
        asset.to_string(),
    )
    .fetch_one(&db)
    .await
    .unwrap();

    Ok((
        [(HX_TRIGGER, "updateWithdrawalAddrs")],
        Html(format!(
            "<p>{address_text}</p>",
            address_text = params.address_text
        )),
    )
        .into_response())
}
