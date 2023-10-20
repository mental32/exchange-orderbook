use std::{collections::HashMap, future::Future, sync::Arc};

use thiserror::Error;

pub mod config;
pub use config::Config;

pub mod asset;
pub use asset::Asset;

pub mod signal;
pub mod trading;
pub mod web;

#[derive(Debug)]
struct Inner {}

#[derive(Debug, Clone)]
pub struct Exchange {
    /// Read-only data or data that has interior mutability.
    inner_ro: Arc<Inner>,
}

impl Exchange {
    pub fn new() -> Self {
        Self {
            inner_ro: Arc::new(Inner {}),
        }
    }

    async fn place_order(
        &self,
        asset: Asset,
        order_type: crate::trading::OrderType,
        stp: crate::trading::SelfTradeProtection,
        side: crate::trading::OrderSide,
    ) {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum StartFullstackError {
    #[error("webserver error")]
    Webserver(#[from] web::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("interrupted")]
    Interrupted,
}

/// Starts the exchange in fullstack mode i.e. all components are ran.
pub fn start_fullstack(
    config: config::Config,
    signals: signal::Signals,
) -> impl Future<Output = Result<(), StartFullstackError>> {
    /// create a future that, depending on the build profile, will either:
    ///
    /// - wait for 1 hour and then resolve
    /// - never resolve
    ///
    /// This has no business purpose, I just have a habit of forgetting to stop
    /// exchange when I'm done developing and I don't want to leave it running
    /// overnight on my laptop.
    ///
    fn automatic_shutdown() -> impl std::future::Future<Output = ()> {
        #[cfg(debug_assertions)]
        return {
            const AUTOMATIC_SHUTDOWN_AFTER_DUR: std::time::Duration =
                std::time::Duration::from_secs(3600); // 1 hour

            tokio::time::sleep(AUTOMATIC_SHUTDOWN_AFTER_DUR)
        };

        #[cfg(not(debug_assertions))]
        return std::future::pending();
    }

    let redis = redis::Client::open(config.redis_url()).expect("Failed to open redis client");

    async move {
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url())
            .await?;

        let (te_tx, rx) = tokio::sync::mpsc::channel(1024);
        let te_input = trading::engine::TradingEngineLoopInput::new(rx);
        let te_handle = std::thread::spawn(move || {
            trading::engine::trading_engine_loop(te_input);
        });

        let assets = Arc::new(HashMap::from_iter(asset::internal_asset_list()));
        let state = web::InternalApiState {
            exchange: Exchange::new(),
            redis,
            db_pool,
            assets,
        };

        let res = tokio::select! {
            res = web::serve(config.webserver_address(), state) => res.map_err(StartFullstackError::Webserver),
            _ = automatic_shutdown() => {
                tracing::info!("auto-shutdown");
                Ok(())
            },
            _ = signals.ctrl_c() => {
                tracing::info!("SIGINT received");
                Err(StartFullstackError::Interrupted)
            },
        };

        // attempt to shutdown gracefully
        tracing::info!("shutting down gracefully");

        // TODO: shutdown gracefully
        te_handle
            .join()
            .expect("Failed to join trading engine thread");

        res
    }
}
