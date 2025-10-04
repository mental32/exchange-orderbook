use anyhow::Context;
use tracing::instrument::WithSubscriber;
use tracing_subscriber::layer::SubscriberExt;

fn main() -> anyhow::Result<()> {
    return tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(
            async move {
                match dotenv::dotenv() {
                    Ok(path) => {
                        tracing::info!(path = %path.display(), "successfully loaded dotenv file")
                    }
                    Err(e) => tracing::error!(?e, "failed loading dotenv file"),
                }

                let config =
                    matching_engine::configuration::Configuration::from_env(std::env::vars())
                        .context("failed loading configuration")?;

                tracing::info!(?config, "loaded configuration");

                let pg_pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(20)
                    .min_connections(1)
                    .connect(&config.database_url)
                    .await
                    .context("could not connect to postgres")?;

                let bitcoind_svc =
                    bitcoind_grpc::connect_bitcoin_rpc(config.bitcoin_grpc_endpoint.clone())
                        .await?;

                let stop_signal = async {
                    let _ = tokio::signal::ctrl_c().await;
                };

                Ok(
                    matching_engine::svc::serve(pg_pool, config.trading_bind_address, stop_signal)
                        .await?,
                )
            }
            .with_subscriber(common_core::tracing::preconfigured_subscriber().finish()),
        );
}
