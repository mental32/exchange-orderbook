//! service layer that exposes the matching engine as a http/rest api

use crate::asset_code::AssetCode;
use crate::asset_code::SymbolVocabulary;
use crate::asset_pair::AssetPairRow;
use crate::asset_pair::BaseQuote;
use crate::svc::ap_actor::launch_ap_procs;
use anyhow::Context as _;
use futures::StreamExt as _;
use futures::stream::FuturesUnordered;
use itertools::Itertools as _;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tracing::Instrument;

pub(in crate::svc) mod ap_actor;
pub(in crate::svc) mod order_management;
pub(in crate::svc) mod routes;

/// Serve the matching engine at the given bind address.
pub async fn serve(
    pg_pool: sqlx::PgPool,
    bind_socket_addr: SocketAddr,
    graceful_shutdown_signal: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let tcp_listener = tokio::net::TcpListener::bind(&bind_socket_addr).await?;

    let (symbol_vocabulary, asset_processors) = launch_ap_procs(pg_pool.clone()).await?;

    let router = common_core::web::apply_middleware(routes::routes(
        order_management::OrderManagement::new(pg_pool, symbol_vocabulary, asset_processors),
    ));

    let make_service = router.into_make_service_with_connect_info::<SocketAddr>();
    let app =
        axum::serve(tcp_listener, make_service).with_graceful_shutdown(graceful_shutdown_signal);
    tracing::info!(?bind_socket_addr, "serving http router for trading");

    Ok(app.await?)
}
