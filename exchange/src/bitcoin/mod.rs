use std::{convert::Infallible, process::ExitCode, time::Duration};

use futures::{future::BoxFuture, FutureExt};

mod client;
pub use client::BitcoinRpcClient;

mod rpc;
use rpc::AddressType;

use crate::{signal::Signals, Config};

pub mod proto {
    tonic::include_proto!("bitcoincore");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("proto_descriptor");
}

struct BitcoinCoreRpcImpl {
    config: Config,
    signals: Signals,
}

impl proto::bitcoin_core_rpc_server::BitcoinCoreRpc for BitcoinCoreRpcImpl {
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_new_address<'life0, 'async_trait>(
        &'life0 self,
        request: tonic::Request<proto::GetNewAddressRequest>,
    ) -> BoxFuture<'async_trait, Result<tonic::Response<proto::GetNewAddressResponse>, tonic::Status>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let proto::GetNewAddressRequest {
            label,
            address_type,
        } = request.into_inner();

        let config = self.config.clone();

        let address_type = match address_type.as_ref().map(|st| st.as_str()) {
            Some("legacy") => Some(AddressType::Legacy),
            Some("p2sh-segwit") => Some(AddressType::P2shSegwit),
            Some("bech32") => Some(AddressType::Bech32),
            Some(_) => {
                return async move {
                    Err(tonic::Status::invalid_argument(
                        "Invalid address type. Valid values are: legacy, p2sh-segwit, bech32",
                    ))
                }
                .boxed()
            }
            None => None,
        };

        async move {
            let (user, pass) = config.bitcoin_rpc_auth();
            let transport = jsonrpc_async::simple_http::SimpleHttpTransport::builder()
                .auth(user, Some(pass))
                .url(config.bitcoin_rpc_url())
                .await
                .expect("Failed to build transport")
                .build();

            let client = jsonrpc_async::client::Client::with_transport(transport);

            let rpc_http = rpc::BitcoinCoreRpcHttp::new(client);

            let label = label.as_ref().map(|st| st.as_str());
            let res = rpc_http.get_new_address(label, address_type).await;

            match res {
                Ok(res) => Ok(tonic::Response::new(proto::GetNewAddressResponse {
                    address: res.to_string(),
                })),
                Err(err) => {
                    tracing::error!(?err);
                    Err(tonic::Status::internal(
                        "Failed to get new address from Bitcoin Core RPC",
                    ))
                }
            }
        }
        .boxed()
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_balance<'life0, 'async_trait>(
        &'life0 self,
        request: tonic::Request<proto::GetNewAddressRequest>,
    ) -> BoxFuture<'async_trait, Result<tonic::Response<proto::GetBalanceResponse>, tonic::Status>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}

pub async fn start_grpc_proxy(
    config: Config,
    signals: Signals,
) -> Result<(), tonic::transport::Error> {
    use proto::bitcoin_core_rpc_server::BitcoinCoreRpcServer;

    let addr = config.bitcoin_grpc_bind_addr();
    tracing::info!(%addr, "starting grpc proxy");

    let svc = BitcoinCoreRpcServer::new(BitcoinCoreRpcImpl { config, signals });

    let reflection: tonic_reflection::server::ServerReflectionServer<_> =
        tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build reflection service");

    tonic::transport::Server::builder()
        .add_service(reflection)
        .add_service(svc)
        .serve_with_shutdown(addr, async move {
            let _ = signals.ctrl_c().await;
            tracing::warn!("SIGINT received");
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(3)).await;
                std::process::exit(1);
            });
        })
        .await
}

/// Connect to a Bitcoin Core RPC server using the given configuration.
///
/// This function will return a client that can be used to make RPC calls to the
/// Bitcoin Core server and depending on the configuration, it will either use
/// jsonrpc over a direct http connection to the bitcoincore node or it will connect to the grpc proxy.
///
pub async fn connect_bitcoin_rpc(config: &Config) -> Result<BitcoinRpcClient, Infallible> {
    Ok(
        BitcoinRpcClient::new_grpc(config.bitcoin_grpc_endpoint().clone())
            .await
            .unwrap(),
    )
}
