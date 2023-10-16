fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body = async {
        tracing_subscriber::fmt::init();
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
