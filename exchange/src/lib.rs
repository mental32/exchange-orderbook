//! exchange-orderbook is an implementation of simple spot exchange. It is a single-process application that runs a webserver and a trading engine.
//!
//! The webserver is responsible for handling user requests and communicating with the trading engine.
//! The trading engine is responsible for maintaining the orderbook and executing trades.
//!
//! # Architecture
//!
//! The exchange is composed of the following components:
//!
//! - [`web`] - the webserver
//! - [`trading`] - the trading engine
//! - [`bitcoin`] - the bitcoin rpc client
//! - [`signal`] - the signal handler
//! - [`config`] - the configuration
//!
//! The exchange can be started in fullstack mode using the `start_everything` function.
//!
#![deny(unused_must_use)]
#![deny(missing_docs)]
#![allow(warnings)]

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use thiserror::Error;
use tracing::Instrument;

pub mod asset;
pub mod bitcoin;
pub mod config;
pub mod jinja;
pub mod signal;
pub mod test;
pub mod trading;
pub mod web;
pub use asset::Asset;
pub use config::Configuration;

pub(crate) mod password;
pub(crate) mod app_cx;
use crate::app_cx::AppCx;

/// Error returned by [`start_fullstack`].
#[derive(Debug, Error)]
pub enum StartFullstackError {
    /// Error returned by the webserver.
    #[error("webserver error")]
    Webserver(#[from] web::ServeError),
    /// Error returned by the database.
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// Error returned by the bitcoin rpc client.
    #[error("bitcoin rpc error: {0}")]
    BitcoinRpc(tonic::transport::Error),
    /// The exchange was interrupted.
    #[error("interrupted")]
    Interrupted,
}

mod spawn_trading_engine;

/// Starts the exchange in fullstack mode i.e. all components are ran.
pub fn start_fullstack(
    config: config::Configuration,
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

    async move {
        tracing::debug!(
            config = ?config,
            "starting exchange in fullstack mode"
        );

        tracing::info!(url = ?config.database_url, "connecting to database");

        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .min_connections(1)
            .connect(&config.database_url)
            .await?;

        tracing::info!("preparing trading engine");

        let btc_rpc = bitcoin::connect_bitcoin_rpc(&config)
            .instrument(tracing::info_span!(
                "bitcoind_rpc_client",
                rpcurl = ?config.bitcoin_rpc_url,
                wallet = ?config.bitcoin_wallet_name,
            ))
            .await
            .map_err(|err| StartFullstackError::BitcoinRpc(err))?;

        let (te_tx, mut te_handle) =
            spawn_trading_engine::spawn_trading_engine(&config, db.clone())
                .init_from_db(db.clone())
                .await?;

        let state = AppCx::new(
            te_tx.clone(),
            btc_rpc,
            db,
            crate::jinja::make_jinja_env(&config),
            config.clone(),
        );

        tracing::info!("launching webserver and waiting for stop signal");

        let res = tokio::select! {
            res = web::serve(config.webserver_bind_addr, state) => res.map_err(StartFullstackError::Webserver),
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
