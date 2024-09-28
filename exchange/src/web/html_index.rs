use axum_htmx::HxRequest;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use mime_guess::MimeGuess;
use minijinja::context;
use thiserror::Error;

use super::InternalApiState;
use super::middleware::auth::try_validate_session;

#[derive(Debug, Error)]
pub enum IndexRouteError {
    #[error("Jinja: {0}")]
    JinjaError(#[from] minijinja::Error),
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for IndexRouteError {
    fn into_response(self) -> axum::response::Response {
        match self {
            IndexRouteError::JinjaError(err) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            IndexRouteError::Sqlx(err) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn f(
    State(state): State<InternalApiState>,
    headers: HeaderMap,
) -> Result<Html<String>, IndexRouteError> {
    let user_uuid = try_validate_session(state.clone(), &headers).await.ok();
    let env = state.jinja().acquire_env()?;
    let render = env.get_template("front-page.html.jinja")?.render(context! { logged_in => user_uuid.is_some() })?;
    Ok(Html(render))
}
