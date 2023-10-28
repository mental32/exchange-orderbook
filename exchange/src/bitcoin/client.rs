use super::proto::bitcoin_core_rpc_client::BitcoinCoreRpcClient;
use super::rpc::BitcoinCoreRpcHttp;
use super::*;

#[derive(Debug, Clone)]
pub struct BitcoinRpcClient(Inner);

#[derive(Debug, Clone)]
enum Inner {
    Grpc(BitcoinCoreRpcClient<tonic::transport::Channel>),
}

impl BitcoinRpcClient {}
