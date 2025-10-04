//! Webserver API for the exchange

use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::LatencyUnit;
use tower_http::ServiceBuilderExt;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::request_id::MakeRequestUuid;
use tower_http::request_id::PropagateRequestIdLayer;
use tower_http::request_id::SetRequestIdLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::DefaultOnFailure;
use tower_http::trace::DefaultOnRequest;
use tower_http::trace::DefaultOnResponse;
use tower_http::trace::TraceLayer;

pub mod middleware;

/// Error returned by the webserver.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum ServeError {
    #[error("axum: {0}")]
    Axum(#[from] axum::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub fn internal_server_error(message: &str) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        message.to_owned(),
    )
        .into_response()
}

pub fn apply_middleware(router: Router) -> Router {
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
            .make_span_with(DefaultMakeSpan::new().include_headers(true)).on_request(DefaultOnRequest::new())
            .on_response(DefaultOnResponse::new().latency_unit(LatencyUnit::Micros))
            .on_failure(DefaultOnFailure::new()),
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

    router.layer(middleware)
}
