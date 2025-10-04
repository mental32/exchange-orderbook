//! Support for bitcoin core rpc.
#![allow(warnings)]

use futures::FutureExt;
use futures::future::BoxFuture;
use rpc::AddressType;
use std::time::Duration;
use tracing::Instrument as _;

use crate::client::BitcoinRpcClient;

pub mod client;
pub mod configuration;
pub mod rpc;

pub mod proto {
    //! Generated code for the protobuf definitions.
    #![allow(missing_docs)]

    tonic::include_proto!("bitcoincore");

    /// The file descriptor set for the protobuf definitions.
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("proto_descriptor");
}

struct BitcoinCoreRpcImpl {
    // config: Configuration,
}

impl proto::bitcoin_core_rpc_server::BitcoinCoreRpc for BitcoinCoreRpcImpl {}

/// Start gRPC server with [`tonic_reflection::server::ServerReflectionServer`] and [`BitcoinCoreRpcImpl`]
pub async fn start_grpc_proxy(
    bitcoin_grpc_bind_addr: std::net::SocketAddr,
) -> Result<(), tonic::transport::Error> {
    use proto::bitcoin_core_rpc_server::BitcoinCoreRpcServer;

    tracing::info!(?bitcoin_grpc_bind_addr, "starting grpc proxy");

    let svc = BitcoinCoreRpcServer::new(BitcoinCoreRpcImpl {});

    let reflection: tonic_reflection::server::ServerReflectionServer<_> =
        tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build reflection service");

    tonic::transport::Server::builder()
        .add_service(reflection)
        .add_service(svc)
        .serve_with_shutdown(bitcoin_grpc_bind_addr, async move {
            let _ = tokio::signal::ctrl_c().await;
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
pub async fn connect_bitcoin_rpc(
    bitcoin_grpc_endpoint: tonic::transport::Endpoint,
) -> Result<BitcoinRpcClient, tonic::transport::Error> {
    tracing::info!(endpoint = ?bitcoin_grpc_endpoint.uri(), "connecting to bitcoin grpc service");
    let bitcoin_rpc_client = BitcoinRpcClient::new_grpc(bitcoin_grpc_endpoint.clone())
        .instrument(tracing::info_span!(
            "bitcoind_rpc_client",
            // rpcurl = ?config.bitcoin_rpc_url,
            // wallet = ?config.bitcoin_wallet_name,
        ))
        .await?;
    Ok(bitcoin_rpc_client)
}
