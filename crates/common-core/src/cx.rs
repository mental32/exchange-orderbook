use anyhow::Context;

use crate::configuration::Configuration;

pub struct Cx {
    config: Configuration,
    pool: sqlx::PgPool,
}

impl Cx {
    pub async fn new(config: Configuration) -> anyhow::Result<Self> {
        tracing::info!(url = ?config.database_url, "connecting to database");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .min_connections(1)
            .connect(&config.database_url)
            .await
            .context("could not connect to postgres")?;

        Ok(Self { config, pool })
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn db(&self) -> sqlx::PgPool {
        self.pool.clone()
    }
}
