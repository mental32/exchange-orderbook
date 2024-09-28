use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{Extension, Form, Json};
use serde::Deserialize;

use crate::bitcoin::proto::GetNewAddressRequest;
use crate::Asset;

use super::middleware::auth::UserUuid;
use super::InternalApiState;

#[derive(Debug, thiserror::Error)]
pub enum CreateDepositAddressError {
    #[error("The specified asset was invalid")]
    InvalidAsset,
    #[error("A deposit address for the specified asset already exists")]
    AlreadyExists,
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for CreateDepositAddressError {
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
pub struct CreateDepositAddressParams {
    asset: String,
}

pub async fn f(
    State(mut state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    Form(params): Form<CreateDepositAddressParams>,
) -> Result<Html<String>, CreateDepositAddressError> {
    let db = state.db();

    // let account = state.fetch_user_account(user_id, params.asset).await?;
    // let account = if let Some(account) = account {
    //     account
    // } else {
    //     state.create_user_account(user_id, asset).await?;
    // } 

    let asset = match params.asset.as_str() {
        "btc" | "BTC" => Asset::Bitcoin,
        "eth" | "ETH" => Asset::Ether,
        _ => {
            tracing::warn!(?params.asset, "invalid asset");
            return Err(CreateDepositAddressError::InvalidAsset);
        }
    };

    let addrs = state.list_deposit_addrs(user_id).await?;
    if addrs
        .iter()
        .any(|(_, asset)| asset.as_str() == params.asset)
    {
        return Err(CreateDepositAddressError::AlreadyExists);
    }

    let address_text: String = match asset {
        Asset::Bitcoin => {
            state
                .bitcoind_rpc
                .get_new_address(GetNewAddressRequest {
                    label: Some(user_id.to_string()),
                    address_type: None,
                })
                .await
                .unwrap()
                .into_inner()
                .address
        }
        Asset::Ether => todo!(),
    };

    let rec = sqlx::query!(
        r#"
        INSERT INTO user_addresses (user_id, address_text, kind, currency)
        VALUES ($1, $2, 'deposit', $3)
        RETURNING id
        "#,
        user_id,
        address_text,
        asset.to_string(),
    )
    .fetch_one(&db)
    .await
    .unwrap();

    Ok(Html(format!("<p>{address_text}</p>")))
}
