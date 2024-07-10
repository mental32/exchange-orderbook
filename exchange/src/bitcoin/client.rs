use tonic::transport::Endpoint;

use super::proto::bitcoin_core_rpc_client::BitcoinCoreRpcClient;

// async fn bitcoind_rpc_client(
//     config: &Config,
// ) -> Result<crate::bitcoin::BitcoinRpcClient, StartFullstackError> {
//     use jsonrpc_async::{self as jsonrpc, simple_http::SimpleHttpTransport};

//     let (username, password) = config.bitcoin_rpc_auth();
//     let transport = SimpleHttpTransport::builder()
//         .auth(username, Some(password))
//         .url(config.bitcoin_rpc_url())
//         .await
//         .unwrap()
//         .build();

//     let client = BitcoinCoreRpc::new(jsonrpc::Client::with_transport(transport));

//     match client.load_wallet(config.bitcoin_wallet_name()).await {
//         Ok(crate::bitcoin::LoadWalletResult { name, warning }) => {
//             tracing::info!(name = ?name, warning = warning, "loaded exchange wallet from remote node");
//         }
//         Err(crate::bitcoin::Error::JsonRpc(jsonrpc_async::Error::Rpc(
//             jsonrpc_async::error::RpcError { message, .. },
//         ))) if message.ends_with("is already loaded.") => {
//             tracing::info!("exchange wallet already loaded");
//         }
//         Err(err) => {
//             tracing::error!(?err, "failed to load exchange wallet from remote node");
//             return Err(StartFullstackError::BitcoinRpc);
//         }
//     };

//     Ok(client)
// }

/// A bitcoin rpc client.
#[derive(Debug, Clone)]
pub struct BitcoinRpcClient(Inner);

#[derive(Debug, Clone)]
enum Inner {
    Grpc(BitcoinCoreRpcClient<tonic::transport::Channel>),
    Mock,
}

impl BitcoinRpcClient {
    /// Create a new bitcoin rpc client.
    pub fn new_grpc(
        endpoint: Endpoint,
    ) -> impl std::future::Future<Output = Result<Self, tonic::transport::Error>> {
        async move {
            let bitcoin_core_rpc_client = BitcoinCoreRpcClient::connect(dbg!(endpoint)).await?;
            tracing::info!("connected to bitcoin core rpc");
            Ok(Self(Inner::Grpc(bitcoin_core_rpc_client)))
        }
    }

    /// Create a dummy client used for testing
    pub fn new_mock() -> Self {
        Self(Inner::Mock)
    }
}
