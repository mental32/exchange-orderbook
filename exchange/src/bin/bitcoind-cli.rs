use std::str::FromStr as _;

use clap::{Parser, Subcommand};
use exchange::bitcoin::rpc::{self, RawTx};

#[derive(Debug, Subcommand)]
enum Command {
    GetNetworkinfo,
    LoadWallet { wallet: String },
    ListWallets,
    GetWalletInfo,
    DumpPrivkey { address: String },
    EncryptWallet,
    GetDifficulty,
    GetConnectioncount,
    GetBlock,
    GetBlockheader,
    GetMiningInfo,
    GetBlockchainInfo,
    GetBlockCount,
    GetBestBlockHash,
    GetBlockHash,
    GetBlockFilter,
    GetBalances,
    SetLabel,
    LockUnspent,
    TestMempoolAccept,
    Stop,
    GetAddressInfo,
    Generate,
    InvalidateBlock,
    ReconsiderBlock,
    GetRawmempool,
    GetMempoolEntry,
    GetPeerinfo,
    Ping,
    SendRawTransaction,
    WaitForNewBlock,
    GetDescriptorInfo { desc: String },
    CombinePsbt,
    GetTxOutsetInfo,
    GetNetTotals,
    Uptime,
    ScanTxOutset,
}

#[derive(Debug, Parser)]
struct Args {
    bitcoind_rpc_url: String,
    user: String,
    pass: String,
    #[command(subcommand)]
    command: Command,
}

fn main() {
    let Args {
        bitcoind_rpc_url,
        user,
        pass,
        command,
    } = Args::parse();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let transport = jsonrpc_async::simple_http::SimpleHttpTransport::builder()
                .auth(user, Some(pass))
                .url(&bitcoind_rpc_url)
                .await
                .expect("Failed to build transport")
                .build();

            let client = rpc::BitcoinCoreRpcHttp::new(
                jsonrpc_async::client::Client::with_transport(transport),
            );

            let st = match command {
                Command::GetNetworkinfo => {
                    let t = client.get_network_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::LoadWallet { wallet } => {
                    let t = client.load_wallet(&wallet).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::ListWallets => {
                    let t = client.list_wallets().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetWalletInfo => {
                    let t = client.get_wallet_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::DumpPrivkey { address } => {
                    let address = bitcoincore_rpc_json::bitcoin::Address::<
                        bitcoincore_rpc_json::bitcoin::address::NetworkUnchecked,
                    >::from_str(&address)
                    .unwrap();
                    let t = client.dump_private_key(&rpc::Address(address)).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::EncryptWallet => todo!(),
                Command::GetDifficulty => {
                    let t = client.get_difficulty().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetConnectioncount => {
                    let t = client.get_connection_count().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetBlock => {
                    let t = client.get_block_info(todo!()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetBlockheader => todo!(),
                Command::GetMiningInfo => {
                    let t = client.get_mining_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetBlockchainInfo => {
                    let t = client.get_blockchain_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetBlockCount => todo!(),
                Command::GetBestBlockHash => todo!(),
                Command::GetBlockHash => todo!(),
                Command::GetBlockFilter => todo!(),
                Command::GetBalances => {
                    let t = client.get_balances().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::SetLabel => todo!(),
                Command::LockUnspent => todo!(),
                Command::TestMempoolAccept => todo!(),
                Command::Stop => todo!(),
                Command::GetAddressInfo => todo!(),
                Command::Generate => todo!(),
                Command::InvalidateBlock => todo!(),
                Command::ReconsiderBlock => todo!(),
                Command::GetRawmempool => todo!(),
                Command::GetMempoolEntry => todo!(),
                Command::GetPeerinfo => {
                    let t = client.get_peer_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::Ping => {
                    let () = client.ping().await.unwrap();
                    serde_json::to_string_pretty(&serde_json::Value::Null).unwrap()
                }
                Command::SendRawTransaction => {
                    let t = client.send_raw_transaction(vec![].as_slice()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::WaitForNewBlock => {
                    let t = client.wait_for_new_block(1000).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetDescriptorInfo {desc} => {
                    let t = client.get_descriptor_info(&desc).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::CombinePsbt => {
                    let t = client.combine_psbt(todo!()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetTxOutsetInfo => {
                    let t = client.get_tx_out_set_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::GetNetTotals => {
                    let t = client.get_net_totals().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::Uptime => {
                    let t = client.uptime().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
                Command::ScanTxOutset => {
                    let t = client.scan_tx_out_set_blocking(vec![].as_slice()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                },
            };

            println!("{st}");
        });
}
