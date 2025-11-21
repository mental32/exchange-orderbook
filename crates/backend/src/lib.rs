//! service layer that exposes the matching engine as a http/rest api
#![allow(warnings)]

use crate::middleware::clerk::ClerkUserId;
use crate::middleware::rate_limit::RateLimitState;
use crate::users::Users;
use anyhow::Context as _;
use ap_actor::proc::launch_processors_for_pairs;
use ap_actor::proc_router::ProcRouter;
use axum::Router;
use axum::extract::FromRef;
use axum::http::header;
use axum::routing::any;
use matching_engine::asset_pair::AssetPairRow;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::LatencyUnit;
use tower_http::ServiceBuilderExt;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::request_id::MakeRequestUuid;
use tower_http::request_id::PropagateRequestIdLayer;
use tower_http::request_id::SetRequestIdLayer;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::DefaultOnFailure;
use tower_http::trace::DefaultOnRequest;
use tower_http::trace::DefaultOnResponse;
use tower_http::trace::TraceLayer;

pub mod middleware;
pub mod secret_str;
pub mod spot_rest;
pub mod spot_websocket_v2;
pub mod users;

#[derive(Debug, Clone, FromRef)]
pub struct AppState {
    proc_router: ProcRouter,
    rate_limit_state: RateLimitState,
    users: Users,
    pg_pool: PgPool,
}

fn apply_middleware(router: Router) -> Router {
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
    // Set x-request-id for response headers.
    .layer(set_request_id_layer)
    .layer(NormalizePathLayer::trim_trailing_slash())
    .layer(PropagateRequestIdLayer::new(x_request_id))
    // Compress responses
    .compression();

    router.layer(middleware)
}

pub async fn serve(
    pg_pool: sqlx::PgPool,
    socket_addr: SocketAddr,
    graceful_shutdown_signal: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let tcp_listener = tokio::net::TcpListener::bind(&socket_addr).await?;

    let t_trading_asset_pairs: Vec<AssetPairRow> = sqlx::query_as!(
        AssetPairRow,
        "SELECT * FROM t_trading_asset_pairs WHERE status = 'active'",
    )
    .fetch_all(&pg_pool)
    .await
    .context("error fetching active asset pairs")?;

    let (symbol_vocabulary, asset_processors) =
        launch_processors_for_pairs(t_trading_asset_pairs, pg_pool.clone()).await?;

    let state = AppState {
        proc_router: ProcRouter::new(symbol_vocabulary, asset_processors),
        rate_limit_state: RateLimitState::default(),
        users: Users::default(),
        pg_pool: pg_pool.clone(),
    };

    let router = apply_middleware(
        Router::new()
            .nest("/0/public/", spot_rest::routes_public(state.clone()))
            .nest("/0/private/", spot_rest::routes_private(state.clone()))
            .route("/v2", any(spot_websocket_v2::f).with_state(state.clone())),
    );

    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let app =
        axum::serve(tcp_listener, make_service).with_graceful_shutdown(graceful_shutdown_signal);
    tracing::info!(?socket_addr, "serving http router for trading");

    Ok(app.await?)
}
