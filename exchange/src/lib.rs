use std::{collections::HashMap, future::Future, sync::Arc};

use sqlx::error;
use thiserror::Error;

pub mod config;
pub use config::Config;

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
        asset: web::Asset,
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
    let redis = redis::Client::open(config.redis_url()).expect("Failed to open redis client");

    async move {
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url())
            .await?;

        let state = web::InternalApiState {
            exchange: Exchange::new(),
            redis,
            db_pool,
            assets: Arc::new({
                let mut map: HashMap<web::asset::InternalAssetKey, web::Asset> = HashMap::new();
                map.insert("BTC".into(), web::Asset::Bitcoin);
                map.insert("btc".into(), web::Asset::Bitcoin);
                map.insert(web::Asset::Bitcoin.into(), web::Asset::Bitcoin);

                map.insert("ETH".into(), web::Asset::Ether);
                map.insert("eth".into(), web::Asset::Ether);
                map.insert(web::Asset::Ether.into(), web::Asset::Ether);

                map
            }),
        };

        let res = tokio::select! {
            res = web::serve(config.webserver_address(), state) => res.map_err(StartFullstackError::Webserver),
            _ = signals.ctrl_c() => {
                tracing::info!("SIGINT received");
                Err(StartFullstackError::Interrupted)
            },
        };

        // attempt to shutdown gracefully
        tracing::info!("shutting down gracefully");

        // TODO: shutdown gracefully

        res
    }
}
