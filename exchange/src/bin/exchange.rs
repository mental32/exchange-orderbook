fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body = async {
        tracing_subscriber::fmt::init();

        let config = exchange::config::Config::load_from_env();

        exchange::web::serve(config.webserver_address(), ()).await
    };

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);
}
