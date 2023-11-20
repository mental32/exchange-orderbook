fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let body = async {
        tracing_subscriber::fmt::fmt()
            .with_file(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        let config = exchange::Config::load_from_env();
        exchange::start_fullstack(config, exchange::signal::from_host_os())
            .await
            .map_err(|err| Box::new(err) as Box<_>)
    };

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);
}
