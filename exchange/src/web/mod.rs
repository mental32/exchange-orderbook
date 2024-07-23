//! Webserver API for the exchange

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
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

mod user_create;
mod user_delete;
mod user_edit;
mod user_get;

mod session_create;
mod session_delete;

mod deposit_list_addrs;
mod deposit_status;

mod withdraw_list_addrs;
mod withdraw_status;
mod withdraw_transfer;

mod public_time;

// mod create_api_key;

mod hx_sign_in {
    use axum::{extract::State, response::Html};

    use super::InternalApiState;

    pub async fn f(
        State(state): State<InternalApiState>,
    ) -> Html<String> {
        todo!()
    }
}

mod hx_sign_up {
    use axum::{extract::State, response::Html};

    use super::InternalApiState;

    pub async fn f(
        State(state): State<InternalApiState>,
    ) -> Html<String> {
        todo!()
    }
}

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

type InternalApiState = crate::app_cx::AppCx;

/// Router for the /trade path
///
/// This router will have the following routes:
/// - `POST /trade/order` - [`trade_add_order`]
/// - `DELETE /trade/order` - [`trade_cancel_order`]
/// - `PUT /trade/order` - [`trade_edit_order`]
///
#[track_caller]
pub fn trade_routes(state: InternalApiState) -> Router {
    let trade_order = post(trade_add_order::f)
        .delete(trade_cancel_order::f)
        .put(trade_edit_order::f);

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
/// - `POST /user` - [`user_create`]
/// - `DELETE /user` - [`user_delete`]
/// - `GET /user` - [`user_get`]
/// - `PUT /user` - [`user_edit`]
///
#[track_caller]
pub fn user_routes(state: InternalApiState) -> Router {
    let user = post(user_create::f)
        .delete(user_delete::f)
        .get(user_get::f)
        .put(user_edit::f);

    Router::new()
        .route("/user", user)
        // .route(
        //     "/user/credentials",
        //     post(create_api_key::api_key_create).route_layer(axum::middleware::from_fn_with_state(
        //         state.clone(),
        //         middleware::validate_session_token,
        //     )),
        // )
        .with_state(state)
}

/// Router for the /deposit path
///
/// This router will have the following routes:
/// - `GET /deposit/addresses` - [`deposit_list_addrs`]
/// - `GET /deposit/status` - [`deposit_status`]
///
#[track_caller]
pub fn deposit_routes(state: InternalApiState) -> Router {
    Router::new()
        .route(
            "/deposit/addresses",
            get(deposit_list_addrs::f),
        )
        .route("/deposit/status/{tx_id}", get(deposit_status::f))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::validate_session_token,
        ))
        .with_state(state)
}

/// Router for the /withdrawal path
///
/// This router will have the following routes:
/// - `GET /withdrawal/addresses` - [`withdraw_list_addrs`]
/// - `GET /withdrawal/status` - [`withdraw_status`]
/// - `POST /withdrawal/transfer` - [`withdraw_transfer`]
///
#[track_caller]
pub fn withdrawal_routes(state: InternalApiState) -> Router {
    Router::new()
        .route(
            "/withdrawal/addresses",
            get(withdraw_list_addrs::f),
        )
        .route("/withdrawal/status/{tx_id}", get(withdraw_status::f))
        // .route(
        //     "/withdrawal/transfer",
        //     axum::routing::post(withdraw_transfer::withdraw_transfer),
        // )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::validate_session_token,
        ))
        .with_state(state)
}

/// Router for the /session path
///
/// This router will have the following routes:
/// - `POST /session` - [`session_create`]
/// - `DELETE /session` - [`session_delete`]
///
#[track_caller]
pub fn session_routes(state: InternalApiState) -> Router {
    let session = post(session_create::f).delete(session_delete::f);

    Router::new().route("/session", session).with_state(state)
}

/// Router for the /public path
pub fn public_routes() -> Router {
    Router::new().route("/public/time", get(public_time::f))
}

fn api_router(state: InternalApiState) -> Router {
    let router = trade_routes(state.clone())
        .merge(user_routes(state.clone()))
        .merge(session_routes(state.clone()))
        .merge(withdrawal_routes(state.clone()))
        .merge(deposit_routes(state.clone()))
        .merge(public_routes());

    Router::new().nest("/api", router)
}

fn hx_router(state: InternalApiState) -> Router {
    let router = Router::new()
        .route("sign-in", get(hx_sign_in::f))
        .with_state(state);

    Router::new().nest("/hx", router)
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

    let router = api_router(state).layer(middleware);

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
