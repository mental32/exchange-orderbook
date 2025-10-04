use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let body = async {
        common_core::tracing::preconfigured_subscriber().init();

        let config = bitcoind_grpc::configuration::Configuration::from_env(std::env::vars());
        bitcoind_grpc::start_grpc_proxy(config.bitcoin_grpc_bind_addr)
            .await
            .map_err(|err| Box::new(err) as Box<_>)
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    let res = runtime.block_on(body);
    runtime.shutdown_timeout(Duration::from_secs(2));
    res
}
