#![deny(unused_must_use)]

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use thiserror::Error;

pub mod config;
pub use config::Config;

pub mod asset;
pub use asset::Asset;
use tracing::Instrument;

use crate::app_cx::AppCx;

pub mod bitcoin;

pub mod signal;
pub mod trading;
pub mod web;

pub(crate) mod app_cx;

#[derive(Debug, Error)]
pub enum StartFullstackError {
    #[error("webserver error")]
    Webserver(#[from] web::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("bitcoin rpc error")]
    BitcoinRpc,
    #[error("interrupted")]
    Interrupted,
}

mod spawn_trading_engine;

async fn bitcoind_rpc_client(
    config: &Config,
) -> Result<crate::bitcoin::BitcoinCoreRpc, StartFullstackError> {
    use crate::bitcoin::BitcoinCoreRpc;
    use jsonrpc_async::{self as jsonrpc, simple_http::SimpleHttpTransport};

    let (username, password) = config.bitcoin_rpc_auth();
    let transport = SimpleHttpTransport::builder()
        .auth(username, Some(password))
        .url(config.bitcoin_rpc_url())
        .await
        .unwrap()
        .build();

    let client = BitcoinCoreRpc::new(jsonrpc::Client::with_transport(transport));

    match client.load_wallet(config.bitcoin_wallet_name()).await {
        Ok(crate::bitcoin::LoadWalletResult { name, warning }) => {
            tracing::info!(name = ?name, warning = warning, "loaded exchange wallet from remote node");
        }
        Err(crate::bitcoin::Error::JsonRpc(jsonrpc_async::Error::Rpc(
            jsonrpc_async::error::RpcError { message, .. },
        ))) if message.ends_with("is already loaded.") => {
            tracing::info!("exchange wallet already loaded");
        }
        Err(err) => {
            tracing::error!(?err, "failed to load exchange wallet from remote node");
            return Err(StartFullstackError::BitcoinRpc);
        }
    };

    Ok(client)
}

/// Starts the exchange in fullstack mode i.e. all components are ran.
pub fn start_fullstack(
    config: config::Config,
    signals: signal::Signals,
) -> impl Future<Output = Result<(), StartFullstackError>> {
    /// create a future that, depending on the build profile, will either:
    ///
    /// - wait for 5 minutes and then resolve (debug)
    /// - never resolve (release)
    ///
    /// This has no real purpose, I just have a habit of forgetting to stop
    /// exchange when I'm done developing and I don't want to leave it running
    /// overnight on my laptop.
    ///
    fn automatic_shutdown() -> impl std::future::Future<Output = ()> {
        #[cfg(debug_assertions)]
        return {
            const AUTOMATIC_SHUTDOWN_AFTER_DUR: std::time::Duration =
                std::time::Duration::from_secs(300); // 5 minutes

            tokio::time::sleep(AUTOMATIC_SHUTDOWN_AFTER_DUR)
        };

        #[cfg(not(debug_assertions))]
        return std::future::pending();
    }

    let redis = redis::Client::open(config.redis_url()).expect("Failed to open redis client");

    async move {
        tracing::debug!(
            config = ?config,
            "starting exchange in fullstack mode"
        );

        tracing::info!(url = ?config.database_url(), "connecting to database");

        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .min_connections(1)
            .connect(&config.database_url())
            .await?;

        tracing::info!("preparing trading engine");

        let bitcoind_rpc = bitcoind_rpc_client(&config)
            .instrument(tracing::info_span!(
                "bitcoind_rpc_client",
                rpcurl = ?config.bitcoin_rpc_url(),
                wallet = ?config.bitcoin_wallet_name(),
            ))
            .await?;

        let (te_tx, mut te_handle) =
            spawn_trading_engine::spawn_trading_engine(&config, db_pool.clone())
                .await
                .initialize_trading_engine(db_pool.clone())
                .await?;

        let assets = Arc::new(HashMap::from_iter(asset::internal_asset_list()));
        let state = web::InternalApiState {
            app_cx: AppCx::new(te_tx.clone(), bitcoind_rpc, db_pool),
            redis,
            assets,
        };

        tracing::info!("launching http server");

        let res = tokio::select! {
            res = web::serve(config.webserver_address(), state) => res.map_err(StartFullstackError::Webserver),
            res = &mut te_handle => match res {
                Ok(()) => {
                    tracing::info!("trading engine shutdown");
                    Ok(())
                },
                Err(err) => {
                    tracing::error!(?err, "trading engine panicked");
                    Err(StartFullstackError::Interrupted)
                }
            },
            () = automatic_shutdown() => {
                tracing::info!("auto-shutdown triggered");
                Ok(())
            },
            _ = signals.ctrl_c() => {
                tracing::info!("SIGINT received");
                Err(StartFullstackError::Interrupted)
            },
        };

        // attempt to shutdown gracefully
        tracing::info!("shutting down gracefully");

        if !te_handle.is_finished() {
            let _ = te_tx.send(trading::TradingEngineCmd::Shutdown).await;

            if let Err(err) = te_handle.await {
                tracing::error!(?err, "trading engine shutdown panicked");
            }
        }

        res
    }
}
