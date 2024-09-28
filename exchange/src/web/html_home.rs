use crate::app_cx::UserDetailsError;

use super::middleware::auth::{try_validate_session, UserUuid};
use super::InternalApiState;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Extension;
use axum_htmx::HxRequest;
use mime_guess::MimeGuess;
use minijinja::context;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HomeRouteError {
    #[error("Jinja: {0}")]
    JinjaError(#[from] minijinja::Error),
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("could not fetch user details: {0}")]
    UserDetailsError(#[from] UserDetailsError),
}

impl IntoResponse for HomeRouteError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(?self);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

fn default_active_tab() -> String {
    "home".into()
}

#[derive(Debug, Deserialize)]
pub struct HomeParams {
    #[serde(default = "default_active_tab")]
    t: String,
}

pub async fn f(
    State(state): State<InternalApiState>,
    Extension(UserUuid(user_id)): Extension<UserUuid>,
    headers: HeaderMap,
    Query(HomeParams { t: tab }): Query<HomeParams>,
) -> Result<Html<String>, HomeRouteError> {
    let mut context =
        context! { user => state.fetch_user_details(user_id).await?, active_tab => tab };

    if tab == "explore" {
        context = context! {
            explore => serde_json::json!({
                "asset_list": [{"name": "BTC"}, {"name": "BTC"}]
            }),
            ..context
        };
    }

    let name = match tab.as_str() {
        "explore" => "consumer/explore.html.jinja",
        "portfolio" => "consumer/portfolio.html.jinja",
        "transactions" => "consumer/transactions.html.jinja",
        "deposit" => "consumer/deposit.html.jinja",
        "withdraw" => "consumer/withdraw.html.jinja",
        "transfer" => "consumer/transfer.html.jinja",
        "home" | _ => "consumer/home.html.jinja",
    };

    let env = state.jinja().acquire_env()?;
    let render = env.get_template(name)?.render(context)?;

    Ok(Html(render))
}
