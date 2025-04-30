use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use thiserror::Error;
use tracing::Instrument;

use crate::app_cx::AppCx;
use crate::cx::Cx;
use crate::web;

/// Error returned by [`start_fullstack`].
#[derive(Debug, Error)]
pub enum StartFullstackError {
    /// Error returned by the webserver.
    #[error("webserver error")]
    Webserver(#[from] crate::web::ServeError),
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

/// Starts the exchange in fullstack mode i.e. all components are ran.
pub fn serve(cx: Cx) -> impl Future<Output = Result<(), StartFullstackError>> {
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
        let config = cx.config();

        tracing::debug!(
            config = ?config,
            "starting exchange in fullstack mode"
        );

        tracing::info!("preparing trading engine");

        let btc_rpc = crate::bitcoin::connect_bitcoin_rpc(&config)
            .instrument(tracing::info_span!(
                "bitcoind_rpc_client",
                rpcurl = ?config.bitcoin_rpc_url,
                wallet = ?config.bitcoin_wallet_name,
            ))
            .await
            .map_err(|err| StartFullstackError::BitcoinRpc(err))?;

        let state = AppCx::new(btc_rpc, cx.db(), config.clone());

        tracing::info!("launching webserver and waiting for stop signal");

        let res = tokio::select! {
            res = web::serve(config.webserver_bind_addr, state) => res.map_err(StartFullstackError::Webserver),
            // _ = signals.ctrl_c() => {
            //     tracing::info!("SIGINT received");
            //     Err(StartFullstackError::Interrupted)
            // },
        };

        // attempt to shutdown gracefully
        tracing::info!("shutting down gracefully");

        res
    }
}
