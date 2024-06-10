//! Webserver API for the exchange

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Router, ServiceExt};

use tokio::net::TcpListener;
use tower::ServiceBuilder;

use tower_http::normalize_path::NormalizePathLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

mod middleware;

mod trade_add_order;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::{LatencyUnit, ServiceBuilderExt};
pub use trade_add_order::TradeAddOrder;
mod trade_cancel_order;
mod trade_edit_order;

mod user_add;
mod user_delete;
mod user_edit;
mod user_get;

mod session_create;
mod session_delete;

mod public_time;

/// Error returned by the webserver.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum ServeError {
    #[error("axum: {0}")]
    Axum(#[from] axum::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

fn internal_server_error(message: &str) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        message.to_owned(),
    )
        .into_response()
}

/// The state of the internal API.
#[derive(Debug, Clone)]
pub struct InternalApiState {
    pub(crate) app_cx: crate::app_cx::AppCx,
    pub(crate) assets: Arc<HashMap<crate::asset::AssetKey, crate::Asset>>,
}

/// Router for the /trade path
///
/// This router will have the following routes:
/// - `POST /trade/order` - [`trade_add_order`]
/// - `DELETE /trade/order` - [`trade_cancel_order`]
/// - `PUT /trade/order` - [`trade_edit_order`]
///
#[track_caller]
pub fn trade_routes(state: InternalApiState) -> Router {
    let trade_order = post(trade_add_order::trade_add_order)
        .delete(trade_cancel_order::trade_cancel_order)
        .put(trade_edit_order::trade_edit_order);

    Router::new()
        .route("/trade/:asset/order", trade_order)
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::validate_session_token,
        ))
        .with_state(state)
}

/// Router for the /user path
///
/// This router will have the following routes:
/// - `POST /user` - [`user_add`]
/// - `DELETE /user` - [`user_delete`]
/// - `GET /user` - [`user_get`]
/// - `PUT /user` - [`user_edit`]
///
#[track_caller]
pub fn user_routes(state: InternalApiState) -> Router {
    let user = post(user_add::user_add)
        .delete(user_delete::user_delete)
        .get(user_get::user_get)
        .put(user_edit::user_edit);

    Router::new().route("/user", user).with_state(state)
}

/// Router for the /session path
///
/// This router will have the following routes:
/// - `POST /session` - [`session_create`]
/// - `DELETE /session` - [`session_delete`]
///
#[track_caller]
pub fn session_routes(state: InternalApiState) -> Router {
    let session = post(session_create::session_create).delete(session_delete::session_delete);

    Router::new().route("/session", session).with_state(state)
}

/// Router for the /public path
pub fn public_routes() -> Router {
    Router::new().route("/public/time", axum::routing::get(public_time::public_time))
}

/// Using [`axum`], serve the internal API on the given address with the provided exchange implementation.
pub fn serve(
    address: SocketAddr,
    state: InternalApiState,
) -> impl Future<Output = Result<(), ServeError>> {
    let x_request_id = axum::http::HeaderName::from_static("x-request-id");

    let set_request_id_layer =
        SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid::default());

    let sensitive_headers: Arc<[_]> = vec![header::AUTHORIZATION, header::COOKIE].into();
    let middleware = ServiceBuilder::new()
    // Mark the `Authorization` and `Cookie` headers as sensitive so it doesn't show in logs
    .sensitive_request_headers(sensitive_headers.clone())
    // Add high level tracing/logging to all requests
    .layer(
        TraceLayer::new_for_http()
            .on_body_chunk(|chunk: &axum::body::Bytes, latency: Duration, _: &tracing::Span| {
                tracing::trace!(size_bytes = chunk.len(), latency = ?latency, "sending body chunk")
            })
            .make_span_with(DefaultMakeSpan::new().include_headers(true))
            .on_response(DefaultOnResponse::new().include_headers(true).latency_unit(LatencyUnit::Micros)),
    )
    .sensitive_response_headers(sensitive_headers)
    // Set a timeout
    .layer(TimeoutLayer::new(Duration::from_secs(10)))
    // Set x-request-id for response headers.
    .layer(set_request_id_layer)
    .layer(NormalizePathLayer::trim_trailing_slash())
    .layer(PropagateRequestIdLayer::new(x_request_id))
    // Compress responses
    .compression();

    let router = trade_routes(state.clone())
        .merge(user_routes(state.clone()))
        .merge(session_routes(state.clone()))
        .merge(public_routes());

    let router = Router::new().nest("/api", router).layer(middleware);

    async move {
        let lst = TcpListener::bind(&address).await?;
        let app = axum::serve(lst, router.into_make_service());
        tracing::info!(?address, "Serving webserver API");
        let rval = app
            .await
            .map_err(axum::Error::new)
            .map_err(ServeError::Axum);
        tracing::warn!(?address, "Stopping webserver!");
        rval
    }
}
