#![allow(warnings)]

use std::str::FromStr as _;

use clap::{Parser, Subcommand};
use exchange::bitcoin::rpc::{self, RawTx};

#[derive(Debug, Subcommand)]
enum Command {
    GetNetworkinfo,
    LoadWallet { wallet: String },
    ListWallets,
    ListTransactions { label: String, count: usize },
    GetWalletInfo,
    DumpPrivkey { address: String },
    EncryptWallet { passphrase: String },
    GetDifficulty,
    GetConnectioncount,
    GetBlock { blockhash: String },
    GetBlockheader { blockhash: String },
    GetMiningInfo,
    GetBlockchainInfo,
    GetBlockCount,
    GetBestBlockHash,
    GetBlockHash { height: u64 },
    GetBlockFilter { blockhash: String },
    GetBalances,
    SetLabel { address: String, label: String },
    LockUnspent { unlock: bool, outputs: Vec<String> },
    TestMempoolAccept { rawtxs: Vec<String> },
    Stop,
    GetAddressInfo { address: String },
    Generate { nblocks: u64, maxtries: Option<u64> },
    InvalidateBlock { blockhash: String },
    ReconsiderBlock { blockhash: String },
    GetRawmempool,
    GetMempoolEntry { txid: String },
    GetPeerinfo,
    Ping,
    SendRawTransaction { hexstring: String },
    WaitForNewBlock,
    GetDescriptorInfo { desc: String },
    CombinePsbt { txs: Vec<String> },
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
                Command::ListTransactions { label, count } => {
                    let t = client
                        .list_transactions(Some(label.as_str()), Some(count as _), None, None)
                        .await
                        .unwrap();
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
                    let t = client
                        .dump_private_key(&rpc::Address(address))
                        .await
                        .unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::EncryptWallet { passphrase } => {
                    let t = client.encrypt_wallet(&passphrase).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetDifficulty => {
                    let t = client.get_difficulty().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetConnectioncount => {
                    let t = client.get_connection_count().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBlock { blockhash } => {
                    let t = client.get_block_info(&blockhash.parse().unwrap()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBlockheader { blockhash } => {
                    todo!()
                    // let t = client.get_block_header(&blockhash.parse().unwrap()).await.unwrap();
                    // serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetMiningInfo => {
                    let t = client.get_mining_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBlockchainInfo => {
                    let t = client.get_blockchain_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBlockCount => {
                    let t = client.get_block_count().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBestBlockHash => {
                    let t = client.get_best_block_hash().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBlockHash { height } => {
                    let t = client.get_block_hash(height).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBlockFilter { blockhash } => {
                    let t = client.get_block_filter(&blockhash.parse().unwrap()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetBalances => {
                    let t = client.get_balances().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::SetLabel { address, label } => {
                    let address = bitcoincore_rpc_json::bitcoin::Address::<
                        bitcoincore_rpc_json::bitcoin::address::NetworkUnchecked,
                    >::from_str(&address)
                    .unwrap();
                    let t = client.set_label(&rpc::Address(address), &label).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::LockUnspent { unlock, outputs } => {
                    let outputs: Vec<_> = outputs
                        .into_iter()
                        .map(|o| serde_json::from_str(&o).unwrap())
                        .collect();
                    if unlock {
                        let t = client.unlock_unspent(&outputs).await.unwrap();
                        serde_json::to_string_pretty(&t).unwrap()

                    } else {
                        let t = client.lock_unspent(&outputs).await.unwrap();
                        serde_json::to_string_pretty(&t).unwrap()
                    }

                }
                Command::TestMempoolAccept { rawtxs } => {
                    let t = client.test_mempool_accept(&rawtxs).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::Stop => {
                    let t = client.stop().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetAddressInfo { address } => {
                    let address = bitcoincore_rpc_json::bitcoin::Address::<
                        bitcoincore_rpc_json::bitcoin::address::NetworkUnchecked,
                    >::from_str(&address)
                    .unwrap();
                    let t = client.get_address_info(&rpc::Address(address)).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::Generate { nblocks, maxtries } => {
                    let t = client.generate(nblocks, maxtries).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::InvalidateBlock { blockhash } => {
                    let t = client.invalidate_block(&blockhash.parse().unwrap()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::ReconsiderBlock { blockhash } => {
                    let t = client.reconsider_block(&blockhash.parse().unwrap()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetRawmempool => {
                    let t = client.get_raw_mempool().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetMempoolEntry { txid } => {
                    let t = client.get_mempool_entry(&txid.parse().unwrap()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetPeerinfo => {
                    let t = client.get_peer_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::Ping => {
                    let () = client.ping().await.unwrap();
                    serde_json::to_string_pretty(&serde_json::Value::Null).unwrap()
                }
                Command::SendRawTransaction { hexstring } => {
                    let t = client.send_raw_transaction(hexstring.as_bytes()).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::WaitForNewBlock => {
                    let t = client.wait_for_new_block(1000).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetDescriptorInfo { desc } => {
                    let t = client.get_descriptor_info(&desc).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::CombinePsbt { txs } => {
                    let t = client.combine_psbt(&txs).await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetTxOutsetInfo => {
                    let t = client.get_tx_out_set_info().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::GetNetTotals => {
                    let t = client.get_net_totals().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::Uptime => {
                    let t = client.uptime().await.unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
                Command::ScanTxOutset => {
                    let t = client
                        .scan_tx_out_set_blocking(&[])
                        .await
                        .unwrap();
                    serde_json::to_string_pretty(&t).unwrap()
                }
            };

            println!("{st}");
        });
}