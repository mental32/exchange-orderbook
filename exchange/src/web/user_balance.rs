use std::collections::HashMap;

use crate::Asset;

use super::middleware::auth::UserUuid;
use super::InternalApiState;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{Extension, Json};

use futures::stream::FuturesUnordered;
use futures::StreamExt;

use serde::Deserialize;
use serde_json::json;

use uuid::Uuid;

pub async fn f(
    State(state): State<InternalApiState>,
    Path(target_user_id): Path<uuid::Uuid>,
    Path(currency): Path<String>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
) -> Response {
    if target_user_id != user_id {
        return StatusCode::FORBIDDEN.into_response();
    }

    if (currency != "*") && (currency.len() != 3) {
        return StatusCode::BAD_REQUEST.into_response()
    }

    state.update_user_accounts(user_id).await;

    let st = if currency == "*" {
        let details = match state.user_balance(user_id).await {
            Ok(t) => t,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        details
            .into_iter()
            .map(|(k, v)| format!("<div id='balance-{k}'>{v}</div>"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let currency = currency.to_ascii_uppercase();

        let balance = match state.calculate_balance_from_accounting(user_id, &currency).await {
            Ok(t) => t.map(|b| b.get()).unwrap_or(0),
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        format!("<div id='balance-{currency}'>{balance}</div>")
    };

    Html(st).into_response()
}
