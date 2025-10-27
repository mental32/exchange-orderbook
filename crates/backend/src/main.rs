use anyhow::Context;
use matching_engine::configuration::Configuration;
use tracing::instrument::WithSubscriber;

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

                let configuration = Configuration::from_map(std::env::vars().collect())
                    .context("failed loading configuration")?;

                tracing::info!(?configuration, "loaded configuration");

                let pg_pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(20)
                    .min_connections(1)
                    .connect(&configuration.database_url)
                    .await
                    .context("could not connect to postgres")?;

                let _bitcoind_rpc_client =
                    bitcoind_grpc::connect_bitcoin_rpc(configuration.bitcoin_grpc_endpoint.clone())
                        .await
                        .context("connecting to bitcoind rpc service")?;

                let stop_signal = async {
                    let _ = tokio::signal::ctrl_c().await;
                };

                Ok(backend::serve(pg_pool, configuration.bind_address, stop_signal).await?)
            }
            .with_subscriber(
                tracing_subscriber::fmt::fmt()
                    .with_file(true)
                    .with_thread_ids(true)
                    .with_line_number(true)
                    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                    .json()
                    .finish(),
            ),
        );
}
