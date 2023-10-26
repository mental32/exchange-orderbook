fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let body = async {
        tracing_subscriber::fmt::fmt()
            .with_file(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .init();

        let config = exchange::Config::load_from_env();
        exchange::bitcoin::start_grpc_proxy(config, exchange::signal::from_host_os())
            .await
            .map_err(|err| Box::new(err) as Box<_>)
    };

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);
}
