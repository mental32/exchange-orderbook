use common_core::cx::Cx;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().unwrap();

    tracing_subscriber::fmt::fmt()
        .with_file(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = common_core::configuration::Configuration::load_from_path(
        common_core::configuration::config_file_path()
            .expect("no config file path")
            .as_path(),
    )?;

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async move {
            let cx = Cx::new(config).await?;
            Ok(
                common_core::fullstack::serve(cx) // config, common_core::signal::from_host_os())
                    .await?,
            )
        });
}
