use anyhow::Context;
use common_core::cx::Cx;

fn main() -> anyhow::Result<()> {
    let dotenv_res = dotenv::dotenv();

    common_core::tracing::init();

    match dotenv_res {
        Ok(path) => tracing::info!(path = %path.display(), "successfully loaded .env file"),
        Err(e) => tracing::error!(?e, "failed loading .env file"),
    }

    let config_file_path =
        common_core::configuration::config_file_path().expect("no config file path");

    let config =
        common_core::configuration::Configuration::load_from_path(config_file_path.as_path())
            .context("failed loading configuration")?;

    tracing::info!(?config, "loaded configuration");

    let future = async move {
        let address = config.trading_bind_address;
        let cx = Cx::new(config).await?;
        Ok(matching_engine::svc::serve(cx.db(), address).await?)
    };

    return tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(future);
}
