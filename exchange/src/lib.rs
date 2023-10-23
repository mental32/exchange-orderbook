use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use futures::StreamExt;
use thiserror::Error;
use tokio::sync::mpsc;

pub mod config;
pub use config::Config;

pub mod asset;
pub use asset::Asset;

pub mod signal;
pub mod trading;
pub mod web;

pub(crate) mod app_cx;
pub(crate) use app_cx::AppCx;

#[derive(Debug, Error)]
pub enum StartFullstackError {
    #[error("webserver error")]
    Webserver(#[from] web::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("interrupted")]
    Interrupted,
}

pub struct SpawnTradingEngine {
    pub input: trading::TradingEngineTx,
    pub handle: tokio::task::JoinHandle<()>,
}

impl SpawnTradingEngine {
    pub async fn initialize_trading_engine(
        self,
        db_pool: sqlx::PgPool,
        redis: redis::Client,
    ) -> Result<(trading::TradingEngineTx, tokio::task::JoinHandle<()>), sqlx::Error> {
        let Self { input, handle } = self;

        tracing::info!("preparing trading engine");

        // stream out rows from the orders_event_source table, deserialize them into TradeCmds
        // and send them to the trading engine for processing.
        let mut stream =
            sqlx::query!(r#"SELECT id, jstr FROM orders_event_source"#,).fetch(&db_pool);

        while let Some(row) = stream.next().await {
            let row = row?;
            let cmd: trading::TradeCmd = serde_json::from_value(row.jstr).unwrap();
            input
                .send(trading::TradingEngineCmd::Trade(cmd))
                .await
                .unwrap();
        }

        Ok((input, handle))
    }
}

async fn spawn_trading_engine(config: &Config) -> SpawnTradingEngine {
    use trading::TradingEngineCmd as T;
    use trading::{AssetBook, Assets};

    async fn trading_engine_supervisor(mut channel_recv: mpsc::Receiver<T>) {
        let mut is_suspended = false;
        let mut assets = Assets {
            eth: AssetBook::new(Asset::Ether),
            btc: AssetBook::new(Asset::Bitcoin),
        };

        while let Some(cmd) = channel_recv.recv().await {
            if is_suspended {
                match cmd {
                    T::Resume => is_suspended = false,
                    _ => continue,
                }
            }

            match cmd {
                T::Shutdown => break,
                T::Suspend => is_suspended = true,
                T::Resume => is_suspended = false,
                T::Trade(cmd) => trading::trading_engine_step(cmd, &mut assets),
            }
        }

        tracing::warn!("trading engine supervisor finished");
    }

    let (input, output) = mpsc::channel(1024);
    let handle = tokio::spawn(trading_engine_supervisor(output));

    SpawnTradingEngine { input, handle }
}

/// Starts the exchange in fullstack mode i.e. all components are ran.
pub fn start_fullstack(
    config: config::Config,
    signals: signal::Signals,
) -> impl Future<Output = Result<(), StartFullstackError>> {
    /// create a future that, depending on the build profile, will either:
    ///
    /// - wait for 1 hour and then resolve (debug)
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
                std::time::Duration::from_secs(3600); // 1 hour

            tokio::time::sleep(AUTOMATIC_SHUTDOWN_AFTER_DUR)
        };

        #[cfg(not(debug_assertions))]
        return std::future::pending();
    }

    let redis = redis::Client::open(config.redis_url()).expect("Failed to open redis client");

    async move {
        tracing::debug!(
            config = toml::to_string(&config).ok(),
            "starting exchange in fullstack mode"
        );

        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url())
            .await?;

        let (te_tx, mut te_handle) = spawn_trading_engine(&config)
            .await
            .initialize_trading_engine(db_pool.clone(), redis.clone())
            .await?;

        tracing::info!("finished preparing trading engine");

        let assets = Arc::new(HashMap::from_iter(asset::internal_asset_list()));
        let state = web::InternalApiState {
            app_cx: AppCx::new(te_tx.clone()),
            redis,
            db_pool,
            assets,
        };

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
