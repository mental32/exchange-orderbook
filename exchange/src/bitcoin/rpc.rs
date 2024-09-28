#![allow(missing_docs)]

use std::fmt::Debug;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::sync::Arc;
use std::{error, fmt, io};

use ahash::HashMap;
use async_trait::async_trait;
use bitcoincore_rpc_json::bitcoin::address::NetworkUnchecked;
use bitcoincore_rpc_json::bitcoin::consensus::encode;
use bitcoincore_rpc_json::bitcoin::hashes::hex::FromHex;
use bitcoincore_rpc_json::bitcoin::hashes::{hex, sha256};
use bitcoincore_rpc_json::bitcoin::hex::HexToBytesError;
use bitcoincore_rpc_json::bitcoin::secp256k1::ecdsa::Signature;
use bitcoincore_rpc_json::bitcoin::sighash::EcdsaSighashType;
use bitcoincore_rpc_json::bitcoin::{
    secp256k1, Amount, OutPoint, PrivateKey, PublicKey, Script, ScriptBuf, SignedAmount,
    Transaction,
};
pub use bitcoincore_rpc_json::{
    bitcoin, AddMultiSigAddressResult, AddressType, Bip9SoftforkInfo, Bip9SoftforkStatistics,
    Bip9SoftforkStatus, BlockRef, CreateRawTransactionInput, EstimateMode, EstimateSmartFeeResult,
    FinalizePsbtResult, FundRawTransactionOptions, FundRawTransactionResult, GetAddressInfoResult,
    GetBalancesResult, GetBlockFilterResult, GetBlockHeaderResult, GetBlockResult,
    GetBlockchainInfoResult, GetDescriptorInfoResult, GetMempoolEntryResult, GetMiningInfoResult,
    GetNetTotalsResult, GetNetworkInfoResult, GetPeerInfoResult, GetRawTransactionResult,
    GetTransactionResult, GetTxOutResult, GetTxOutSetInfoResult, GetWalletInfoResult,
    ImportMultiOptions, ImportMultiRequest, ImportMultiResult, ListReceivedByAddressResult,
    ListSinceBlockResult, ListTransactionResult, ListUnspentQueryOptions, ListUnspentResultEntry,
    LoadWalletResult, PubKeyOrAddress, ScanTxOutRequest, ScanTxOutResult, SignRawTransactionInput,
    SignRawTransactionResult, Softfork, SoftforkType, TestMempoolAcceptResult,
    WalletCreateFundedPsbtOptions, WalletCreateFundedPsbtResult,
};
use jsonrpc_async;
use rustc_hex::ToHex;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub bitcoincore_rpc_json::bitcoin::Address<NetworkUnchecked>);

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.clone().assume_checked(), f)
    }
}

/// The error type for errors produced in this library.
#[derive(Debug)]
pub enum Error {
    JsonRpc(jsonrpc_async::error::Error),
    Hex(HexToBytesError),
    Json(serde_json::error::Error),
    BitcoinSerialization(bitcoin::consensus::encode::Error),
    Secp256k1(secp256k1::Error),
    Io(io::Error),
    InvalidAmount(bitcoin::amount::ParseAmountError),
    InvalidCookieFile,
    /// The JSON result had an unexpected structure.
    UnexpectedStructure,
}

impl From<jsonrpc_async::error::Error> for Error {
    fn from(e: jsonrpc_async::error::Error) -> Error {
        Error::JsonRpc(e)
    }
}

impl From<HexToBytesError> for Error {
    fn from(e: HexToBytesError) -> Error {
        Error::Hex(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Error {
        Error::Json(e)
    }
}

impl From<bitcoin::consensus::encode::Error> for Error {
    fn from(e: bitcoin::consensus::encode::Error) -> Error {
        Error::BitcoinSerialization(e)
    }
}

impl From<secp256k1::Error> for Error {
    fn from(e: secp256k1::Error) -> Error {
        Error::Secp256k1(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<bitcoin::amount::ParseAmountError> for Error {
    fn from(e: bitcoin::amount::ParseAmountError) -> Error {
        Error::InvalidAmount(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::JsonRpc(ref e) => write!(f, "JSON-RPC error: {}", e),
            Error::Hex(ref e) => write!(f, "hex decode error: {}", e),
            Error::Json(ref e) => write!(f, "JSON error: {}", e),
            Error::BitcoinSerialization(ref e) => write!(f, "Bitcoin serialization error: {}", e),
            Error::Secp256k1(ref e) => write!(f, "secp256k1 error: {}", e),
            Error::Io(ref e) => write!(f, "I/O error: {}", e),
            Error::InvalidAmount(ref e) => write!(f, "invalid amount: {}", e),
            Error::InvalidCookieFile => write!(f, "invalid cookie file"),
            Error::UnexpectedStructure => write!(f, "the JSON result had an unexpected structure"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "bitcoincore-rpc error"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::JsonRpc(ref e) => Some(e),
            Error::Json(ref e) => Some(e),
            Error::BitcoinSerialization(ref e) => Some(e),
            Error::Secp256k1(ref e) => Some(e),
            Error::Io(ref e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JsonOutPoint {
    pub txid: bitcoin::Txid,
    pub vout: u32,
}

impl From<OutPoint> for JsonOutPoint {
    fn from(o: OutPoint) -> JsonOutPoint {
        JsonOutPoint {
            txid: o.txid,
            vout: o.vout,
        }
    }
}

impl From<JsonOutPoint> for OutPoint {
    fn from(jop: JsonOutPoint) -> OutPoint {
        OutPoint {
            txid: jop.txid,
            vout: jop.vout,
        }
    }
}

/// Shorthand for converting a variable into a serde_json::Value.
fn into_json<T>(val: T) -> Result<serde_json::Value, Error>
where
    T: serde::ser::Serialize,
{
    Ok(serde_json::to_value(val)?)
}

/// Shorthand for converting an Option into an Option<serde_json::Value>.
fn opt_into_json<T>(opt: Option<T>) -> Result<serde_json::Value, Error>
where
    T: serde::ser::Serialize,
{
    match opt {
        Some(val) => Ok(into_json(val)?),
        None => Ok(serde_json::Value::Null),
    }
}

/// Shorthand for `serde_json::Value::Null`.
fn null() -> serde_json::Value {
    serde_json::Value::Null
}

/// Shorthand for an empty serde_json::Value array.
fn empty_arr() -> serde_json::Value {
    serde_json::Value::Array(vec![])
}

/// Shorthand for an empty serde_json object.
fn empty_obj() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}

/// Handle default values in the argument list
///
/// Substitute `Value::Null`s with corresponding values from `defaults` table,
/// except when they are trailing, in which case just skip them altogether
/// in returned list.
///
/// Note, that `defaults` corresponds to the last elements of `args`.
///
/// ```norust
/// arg1 arg2 arg3 arg4
///           def1 def2
/// ```
///
/// Elements of `args` without corresponding `defaults` value, won't
/// be substituted, because they are required.
fn handle_defaults<'a, 'b>(
    args: &'a mut [serde_json::Value],
    defaults: &'b [serde_json::Value],
) -> &'a [serde_json::Value] {
    assert!(args.len() >= defaults.len());

    // Pass over the optional arguments in backwards order, filling in defaults after the first
    // non-null optional argument has been observed.
    let mut first_non_null_optional_idx = None;
    for i in 0..defaults.len() {
        let args_i = args.len() - 1 - i;
        let defaults_i = defaults.len() - 1 - i;
        if args[args_i] == serde_json::Value::Null {
            if first_non_null_optional_idx.is_some() {
                if defaults[defaults_i] == serde_json::Value::Null {
                    panic!("Missing `default` for argument idx {}", args_i);
                }
                args[args_i] = defaults[defaults_i].clone();
            }
        } else if first_non_null_optional_idx.is_none() {
            first_non_null_optional_idx = Some(args_i);
        }
    }

    let required_num = args.len() - defaults.len();

    if let Some(i) = first_non_null_optional_idx {
        &args[..i + 1]
    } else {
        &args[..required_num]
    }
}

/// Convert a possible-null result into an Option.
fn opt_result<T: for<'a> serde::de::Deserialize<'a>>(
    result: serde_json::Value,
) -> Result<Option<T>, Error> {
    if result == serde_json::Value::Null {
        Ok(None)
    } else {
        Ok(serde_json::from_value(result)?)
    }
}

/// Used to pass raw txs into the API.
pub trait RawTx: Sized + Clone {
    fn raw_hex(self) -> String;
}

impl<'a> RawTx for &'a Transaction {
    fn raw_hex(self) -> String {
        bitcoincore_rpc_json::bitcoin::consensus::encode::serialize(self).to_hex()
    }
}

impl<'a> RawTx for &'a [u8] {
    fn raw_hex(self) -> String {
        self.to_hex()
    }
}

impl<'a> RawTx for &'a Vec<u8> {
    fn raw_hex(self) -> String {
        self.to_hex()
    }
}

impl<'a> RawTx for &'a str {
    fn raw_hex(self) -> String {
        self.to_owned()
    }
}

impl RawTx for String {
    fn raw_hex(self) -> String {
        self
    }
}

/// The different authentication methods for the client.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Auth {
    None,
    UserPass(String, String),
    CookieFile(PathBuf),
}

impl Auth {
    // /// Convert into the arguments that jsonrpc_async::Client needs.
    // fn get_user_pass(self) -> Result<Option<(String, String)>, Error> {
    //     use std::io::Read;
    //     match self {
    //         Auth::None => Ok(None),
    //         Auth::UserPass(u, p) => Ok(Some((u, p))),
    //         Auth::CookieFile(path) => {
    //             let mut file = File::open(path)?;
    //             let mut contents = String::new();
    //             file.read_to_string(&mut contents)?;
    //             let mut split = contents.splitn(2, ':');
    //             let u = split.next().ok_or(Error::InvalidCookieFile)?.into();
    //             let p = split.next().ok_or(Error::InvalidCookieFile)?.into();
    //             Ok(Some((u, p)))
    //         }
    //     }
    // }
}

/// Client implements a JSON-RPC client for the Bitcoin Core daemon or compatible APIs.
#[derive(Clone)]
pub struct BitcoinCoreRpcHttp {
    client: Arc<jsonrpc_async::client::Client>,
}

impl fmt::Debug for BitcoinCoreRpcHttp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bitcoincore_rpc::Client(jsonrpc_async::client::Client(last_nonce=?))",
        )
    }
}

impl BitcoinCoreRpcHttp {
    pub fn new(client: jsonrpc_async::client::Client) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

impl BitcoinCoreRpcHttp {
    /// Call an `cmd` rpc with given `args` list
    async fn send_request<T: for<'a> serde::de::Deserialize<'a>>(
        &self,
        cmd: &str,
        args: &[serde_json::Value],
    ) -> Result<T, Error> {
        let v_args: Vec<_> = args
            .iter()
            .map(serde_json::value::to_raw_value)
            .collect::<std::result::Result<_, serde_json::Error>>()?;
        let req = self.client.build_request(cmd, &v_args[..]);

        let resp = self.client.send_request(req).await.map_err(Error::from);
        Ok(resp?.result()?)
    }

    pub async fn get_network_info(&self) -> Result<GetNetworkInfoResult, Error> {
        self.send_request("getnetworkinfo", &[]).await
    }

    pub async fn version(&self) -> Result<usize, Error> {
        #[derive(Deserialize)]
        struct Response {
            pub version: usize,
        }
        let res: Response = self.send_request("getnetworkinfo", &[]).await?;
        Ok(res.version)
    }

    pub async fn add_multisig_address(
        &self,
        nrequired: usize,
        keys: &[PubKeyOrAddress<'_>],
        label: Option<&str>,
        address_type: Option<AddressType>,
    ) -> Result<AddMultiSigAddressResult, Error> {
        let mut args = [
            into_json(nrequired)?,
            into_json(keys)?,
            opt_into_json(label)?,
            opt_into_json(address_type)?,
        ];
        self.send_request(
            "addmultisigaddress",
            handle_defaults(&mut args, &[into_json("")?, null()]),
        )
        .await
    }

    pub async fn load_wallet(&self, wallet: &str) -> Result<LoadWalletResult, Error> {
        self.send_request("loadwallet", &[wallet.into()]).await
    }

    pub async fn unload_wallet(&self, wallet: Option<&str>) -> Result<(), Error> {
        let mut args = [opt_into_json(wallet)?];
        self.send_request("unloadwallet", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn create_wallet(
        &self,
        wallet: &str,
        disable_private_keys: Option<bool>,
        blank: Option<bool>,
        passphrase: Option<&str>,
        avoid_reuse: Option<bool>,
    ) -> Result<LoadWalletResult, Error> {
        let mut args = [
            wallet.into(),
            opt_into_json(disable_private_keys)?,
            opt_into_json(blank)?,
            opt_into_json(passphrase)?,
            opt_into_json(avoid_reuse)?,
        ];
        self.send_request(
            "createwallet",
            handle_defaults(
                &mut args,
                &[false.into(), false.into(), into_json("")?, false.into()],
            ),
        )
        .await
    }

    pub async fn list_wallets(&self) -> Result<Vec<String>, Error> {
        self.send_request("listwallets", &[]).await
    }

    pub async fn get_wallet_info(&self) -> Result<GetWalletInfoResult, Error> {
        self.send_request("getwalletinfo", &[]).await
    }

    pub async fn backup_wallet(&self, destination: Option<&str>) -> Result<(), Error> {
        let mut args = [opt_into_json(destination)?];
        self.send_request("backupwallet", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn dump_private_key(&self, address: &Address) -> Result<PrivateKey, Error> {
        self.send_request("dumpprivkey", &[address.to_string().into()])
            .await
    }

    pub async fn encrypt_wallet(&self, passphrase: &str) -> Result<(), Error> {
        self.send_request("encryptwallet", &[into_json(passphrase)?])
            .await
    }

    pub async fn get_difficulty(&self) -> Result<f64, Error> {
        self.send_request("getdifficulty", &[]).await
    }

    pub async fn get_connection_count(&self) -> Result<usize, Error> {
        self.send_request("getconnectioncount", &[]).await
    }

    // pub async fn get_block(&self, hash: &bitcoin::BlockHash) -> Result<Block, Error> {
    //     let hex: String = self.call("getblock", &[into_json(hash)?, 0.into()]).await?;
    //     let bytes: Vec<u8> = FromHex::from_hex(&hex)?;
    //     Ok(bitcoin::consensus::encode::deserialize(&bytes)?)
    // }

    pub async fn get_block_hex(&self, hash: &bitcoin::BlockHash) -> Result<String, Error> {
        self.send_request("getblock", &[into_json(hash)?, 0.into()])
            .await
    }

    pub async fn get_block_info(&self, hash: &bitcoin::BlockHash) -> Result<GetBlockResult, Error> {
        self.send_request("getblock", &[into_json(hash)?, 1.into()])
            .await
    }
    //TODO(stevenroose) add getblock_txs

    // pub async fn get_block_header(&self, hash: &bitcoin::BlockHash) -> Result<BlockHeader, Error> {
    //     let hex: String = self
    //         .call("getblockheader", &[into_json(hash)?, false.into()])
    //         .await?;
    //     let bytes: Vec<u8> = FromHex::from_hex(&hex)?;
    //     Ok(bitcoin::consensus::encode::deserialize(&bytes)?)
    // }

    pub async fn get_block_header_info(
        &self,
        hash: &bitcoin::BlockHash,
    ) -> Result<GetBlockHeaderResult, Error> {
        self.send_request("getblockheader", &[into_json(hash)?, true.into()])
            .await
    }

    pub async fn get_mining_info(&self) -> Result<GetMiningInfoResult, Error> {
        self.send_request("getmininginfo", &[]).await
    }

    /// Returns a data structure containing various state info regarding
    /// blockchain processing.
    pub async fn get_blockchain_info(&self) -> Result<GetBlockchainInfoResult, Error> {
        let mut raw: serde_json::Value = self.send_request("getblockchaininfo", &[]).await?;
        // The softfork fields are not backwards compatible:
        // - 0.18.x returns a "softforks" array and a "bip9_softforks" map.
        // - 0.19.x returns a "softforks" map.
        Ok(if self.version().await? < 190000 {
            use Error::UnexpectedStructure as err;

            // First, remove both incompatible softfork fields.
            // We need to scope the mutable ref here for v1.29 borrowck.
            let (bip9_softforks, old_softforks) = {
                let map = raw.as_object_mut().ok_or(err)?;
                let bip9_softforks = map.remove("bip9_softforks").ok_or(err)?;
                let old_softforks = map.remove("softforks").ok_or(err)?;
                // Put back an empty "softforks" field.
                map.insert("softforks".into(), serde_json::Map::new().into());
                (bip9_softforks, old_softforks)
            };
            let mut ret: GetBlockchainInfoResult = serde_json::from_value(raw)?;

            // Then convert both softfork types and add them.
            for sf in old_softforks.as_array().ok_or(err)?.iter() {
                let json = sf.as_object().ok_or(err)?;
                let id = json.get("id").ok_or(err)?.as_str().ok_or(err)?;
                let reject = json.get("reject").ok_or(err)?.as_object().ok_or(err)?;
                let active = reject.get("status").ok_or(err)?.as_bool().ok_or(err)?;
                ret.softforks.insert(
                    id.into(),
                    Softfork {
                        type_: SoftforkType::Buried,
                        bip9: None,
                        height: None,
                        active,
                    },
                );
            }
            for (id, sf) in bip9_softforks.as_object().ok_or(err)?.iter() {
                #[derive(Deserialize)]
                struct OldBip9SoftFork {
                    pub status: Bip9SoftforkStatus,
                    pub bit: Option<u8>,
                    #[serde(rename = "startTime")]
                    pub start_time: i64,
                    pub timeout: u64,
                    pub since: u32,
                    pub statistics: Option<Bip9SoftforkStatistics>,
                }
                let sf: OldBip9SoftFork = serde_json::from_value(sf.clone())?;
                ret.softforks.insert(
                    id.clone(),
                    Softfork {
                        type_: SoftforkType::Bip9,
                        bip9: Some(Bip9SoftforkInfo {
                            status: sf.status,
                            bit: sf.bit,
                            start_time: sf.start_time,
                            timeout: sf.timeout,
                            since: sf.since,
                            statistics: sf.statistics,
                        }),
                        height: None,
                        active: sf.status == Bip9SoftforkStatus::Active,
                    },
                );
            }
            ret
        } else {
            serde_json::from_value(raw)?
        })
    }

    /// Returns the numbers of block in the longest chain.
    pub async fn get_block_count(&self) -> Result<u64, Error> {
        self.send_request("getblockcount", &[]).await
    }

    /// Returns the hash of the best (tip) block in the longest blockchain.
    pub async fn get_best_block_hash(&self) -> Result<bitcoin::BlockHash, Error> {
        self.send_request("getbestblockhash", &[]).await
    }

    /// Get block hash at a given height
    pub async fn get_block_hash(&self, height: u64) -> Result<bitcoin::BlockHash, Error> {
        self.send_request("getblockhash", &[height.into()]).await
    }

    pub async fn get_raw_transaction(
        &self,
        txid: &bitcoin::Txid,
        block_hash: Option<&bitcoin::BlockHash>,
    ) -> Result<Transaction, Error> {
        let mut args = [
            into_json(txid)?,
            into_json(false)?,
            opt_into_json(block_hash)?,
        ];
        let hex: String = self
            .send_request("getrawtransaction", handle_defaults(&mut args, &[null()]))
            .await?;
        let bytes: Vec<u8> = FromHex::from_hex(&hex)?;
        Ok(bitcoin::consensus::encode::deserialize(&bytes)?)
    }

    pub async fn get_raw_transaction_hex(
        &self,
        txid: &bitcoin::Txid,
        block_hash: Option<&bitcoin::BlockHash>,
    ) -> Result<String, Error> {
        let mut args = [
            into_json(txid)?,
            into_json(false)?,
            opt_into_json(block_hash)?,
        ];
        self.send_request("getrawtransaction", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn get_raw_transaction_info(
        &self,
        txid: &bitcoin::Txid,
        block_hash: Option<&bitcoin::BlockHash>,
    ) -> Result<GetRawTransactionResult, Error> {
        let mut args = [
            into_json(txid)?,
            into_json(true)?,
            opt_into_json(block_hash)?,
        ];
        self.send_request("getrawtransaction", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn get_block_filter(
        &self,
        block_hash: &bitcoin::BlockHash,
    ) -> Result<GetBlockFilterResult, Error> {
        self.send_request("getblockfilter", &[into_json(block_hash)?])
            .await
    }

    pub async fn get_balance(
        &self,
        minconf: Option<usize>,
        include_watchonly: Option<bool>,
    ) -> Result<Amount, Error> {
        let mut args = [
            "*".into(),
            opt_into_json(minconf)?,
            opt_into_json(include_watchonly)?,
        ];
        Ok(Amount::from_btc(
            self.send_request(
                "getbalance",
                handle_defaults(&mut args, &[0.into(), null()]),
            )
            .await?,
        )?)
    }

    pub async fn get_balances(&self) -> Result<GetBalancesResult, Error> {
        Ok(self.send_request("getbalances", &[]).await?)
    }

    pub async fn get_received_by_address(
        &self,
        address: &Address,
        minconf: Option<u32>,
    ) -> Result<Amount, Error> {
        let mut args = [address.to_string().into(), opt_into_json(minconf)?];
        Ok(Amount::from_btc(
            self.send_request(
                "getreceivedbyaddress",
                handle_defaults(&mut args, &[null()]),
            )
            .await?,
        )?)
    }

    pub async fn get_transaction(
        &self,
        txid: &bitcoin::Txid,
        include_watchonly: Option<bool>,
    ) -> Result<GetTransactionResult, Error> {
        let mut args = [into_json(txid)?, opt_into_json(include_watchonly)?];
        self.send_request("gettransaction", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn list_transactions(
        &self,
        label: Option<&str>,
        count: Option<usize>,
        skip: Option<usize>,
        include_watchonly: Option<bool>,
    ) -> Result<Vec<ListTransactionResult>, Error> {
        let mut args = [
            label.unwrap_or("*").into(),
            opt_into_json(count)?,
            opt_into_json(skip)?,
            opt_into_json(include_watchonly)?,
        ];
        self.send_request(
            "listtransactions",
            handle_defaults(&mut args, &[10.into(), 0.into(), null()]),
        )
        .await
    }

    pub async fn list_since_block(
        &self,
        blockhash: Option<&bitcoin::BlockHash>,
        target_confirmations: Option<usize>,
        include_watchonly: Option<bool>,
        include_removed: Option<bool>,
    ) -> Result<ListSinceBlockResult, Error> {
        let mut args = [
            opt_into_json(blockhash)?,
            opt_into_json(target_confirmations)?,
            opt_into_json(include_watchonly)?,
            opt_into_json(include_removed)?,
        ];
        self.send_request("listsinceblock", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn get_tx_out(
        &self,
        txid: &bitcoin::Txid,
        vout: u32,
        include_mempool: Option<bool>,
    ) -> Result<Option<GetTxOutResult>, Error> {
        let mut args = [
            into_json(txid)?,
            into_json(vout)?,
            opt_into_json(include_mempool)?,
        ];
        opt_result(
            self.send_request("gettxout", handle_defaults(&mut args, &[null()]))
                .await?,
        )
    }

    pub async fn get_tx_out_proof(
        &self,
        txids: &[bitcoin::Txid],
        block_hash: Option<&bitcoin::BlockHash>,
    ) -> Result<Vec<u8>, Error> {
        let mut args = [into_json(txids)?, opt_into_json(block_hash)?];
        let hex: String = self
            .send_request("gettxoutproof", handle_defaults(&mut args, &[null()]))
            .await?;
        Ok(FromHex::from_hex(&hex)?)
    }

    pub async fn import_public_key(
        &self,
        pubkey: &PublicKey,
        label: Option<&str>,
        rescan: Option<bool>,
    ) -> Result<(), Error> {
        let mut args = [
            pubkey.to_string().into(),
            opt_into_json(label)?,
            opt_into_json(rescan)?,
        ];
        self.send_request(
            "importpubkey",
            handle_defaults(&mut args, &[into_json("")?, null()]),
        )
        .await
    }

    pub async fn import_private_key(
        &self,
        privkey: &PrivateKey,
        label: Option<&str>,
        rescan: Option<bool>,
    ) -> Result<(), Error> {
        let mut args = [
            privkey.to_string().into(),
            opt_into_json(label)?,
            opt_into_json(rescan)?,
        ];
        self.send_request(
            "importprivkey",
            handle_defaults(&mut args, &[into_json("")?, null()]),
        )
        .await
    }

    pub async fn import_address(
        &self,
        address: &Address,
        label: Option<&str>,
        rescan: Option<bool>,
    ) -> Result<(), Error> {
        let mut args = [
            address.to_string().into(),
            opt_into_json(label)?,
            opt_into_json(rescan)?,
        ];
        self.send_request(
            "importaddress",
            handle_defaults(&mut args, &[into_json("")?, null()]),
        )
        .await
    }

    pub async fn import_address_script(
        &self,
        script: &Script,
        label: Option<&str>,
        rescan: Option<bool>,
        p2sh: Option<bool>,
    ) -> Result<(), Error> {
        let mut args = [
            script.to_hex_string().into(),
            opt_into_json(label)?,
            opt_into_json(rescan)?,
            opt_into_json(p2sh)?,
        ];
        self.send_request(
            "importaddress",
            handle_defaults(&mut args, &[into_json("")?, true.into(), null()]),
        )
        .await
    }

    pub async fn import_multi(
        &self,
        requests: &[ImportMultiRequest<'_>],
        options: Option<&ImportMultiOptions>,
    ) -> Result<Vec<ImportMultiResult>, Error> {
        let mut json_requests = Vec::with_capacity(requests.len());
        for req in requests {
            json_requests.push(serde_json::to_value(req)?);
        }
        let mut args = [json_requests.into(), opt_into_json(options)?];
        self.send_request("importmulti", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn set_label(&self, address: &Address, label: &str) -> Result<(), Error> {
        self.send_request("setlabel", &[address.to_string().into(), label.into()])
            .await
    }

    pub async fn key_pool_refill(&self, new_size: Option<usize>) -> Result<(), Error> {
        let mut args = [opt_into_json(new_size)?];
        self.send_request("keypoolrefill", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn list_unspent(
        &self,
        minconf: Option<usize>,
        maxconf: Option<usize>,
        addresses: Option<&[&Address]>,
        include_unsafe: Option<bool>,
        query_options: Option<ListUnspentQueryOptions>,
    ) -> Result<Vec<ListUnspentResultEntry>, Error> {
        let mut args = [
            opt_into_json(minconf)?,
            opt_into_json(maxconf)?,
            opt_into_json(addresses)?,
            opt_into_json(include_unsafe)?,
            opt_into_json(query_options)?,
        ];
        let defaults = [
            into_json(0)?,
            into_json(9999999)?,
            empty_arr(),
            into_json(true)?,
            null(),
        ];
        self.send_request("listunspent", handle_defaults(&mut args, &defaults))
            .await
    }

    /// To unlock, use [unlock_unspent].
    pub async fn lock_unspent(&self, outputs: &[OutPoint]) -> Result<bool, Error> {
        let outputs: Vec<_> = outputs
            .iter()
            .map(|o| serde_json::to_value(JsonOutPoint::from(*o)).unwrap())
            .collect();
        self.send_request("lockunspent", &[false.into(), outputs.into()])
            .await
    }

    pub async fn unlock_unspent(&self, outputs: &[OutPoint]) -> Result<bool, Error> {
        let outputs: Vec<_> = outputs
            .iter()
            .map(|o| serde_json::to_value(JsonOutPoint::from(*o)).unwrap())
            .collect();
        self.send_request("lockunspent", &[true.into(), outputs.into()])
            .await
    }

    pub async fn list_received_by_address(
        &self,
        address_filter: Option<Address>,
        minconf: Option<u32>,
        include_empty: Option<bool>,
        include_watchonly: Option<bool>,
    ) -> Result<Vec<ListReceivedByAddressResult>, Error> {
        let mut args = [
            opt_into_json(minconf)?,
            opt_into_json(include_empty)?,
            opt_into_json(include_watchonly)?,
            opt_into_json(address_filter)?,
        ];
        let defaults = [1.into(), false.into(), false.into(), null()];
        self.send_request(
            "listreceivedbyaddress",
            handle_defaults(&mut args, &defaults),
        )
        .await
    }

    pub async fn create_raw_transaction_hex(
        &self,
        utxos: &[CreateRawTransactionInput],
        outs: &HashMap<String, Amount>,
        locktime: Option<i64>,
        replaceable: Option<bool>,
    ) -> Result<String, Error> {
        let outs_converted = serde_json::Map::from_iter(
            outs.iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::from(v.to_btc()))),
        );
        let mut args = [
            into_json(utxos)?,
            into_json(outs_converted)?,
            opt_into_json(locktime)?,
            opt_into_json(replaceable)?,
        ];
        let defaults = [into_json(0i64)?, null()];
        self.send_request(
            "createrawtransaction",
            handle_defaults(&mut args, &defaults),
        )
        .await
    }

    pub async fn create_raw_transaction(
        &self,
        utxos: &[CreateRawTransactionInput],
        outs: &HashMap<String, Amount>,
        locktime: Option<i64>,
        replaceable: Option<bool>,
    ) -> Result<Transaction, Error> {
        let hex: String = self
            .create_raw_transaction_hex(utxos, outs, locktime, replaceable)
            .await?;
        let bytes: Vec<u8> = FromHex::from_hex(&hex)?;
        Ok(bitcoin::consensus::encode::deserialize(&bytes)?)
    }

    pub async fn fund_raw_transaction<R: RawTx>(
        &self,
        tx: R,
        options: Option<&FundRawTransactionOptions>,
        is_witness: Option<bool>,
    ) -> Result<FundRawTransactionResult, Error>
    where
        R: Sync + Send,
    {
        let mut args = [
            tx.raw_hex().into(),
            opt_into_json(options)?,
            opt_into_json(is_witness)?,
        ];
        let defaults = [empty_obj(), null()];
        self.send_request("fundrawtransaction", handle_defaults(&mut args, &defaults))
            .await
    }

    #[deprecated]
    pub async fn sign_raw_transaction<R: RawTx>(
        &self,
        tx: R,
        utxos: Option<&[SignRawTransactionInput]>,
        private_keys: Option<&[PrivateKey]>,
        sighash_type: Option<EcdsaSighashType>,
    ) -> Result<SignRawTransactionResult, Error>
    where
        R: Sync + Send,
    {
        let mut args = [
            tx.raw_hex().into(),
            opt_into_json(utxos)?,
            opt_into_json(private_keys)?,
            opt_into_json(sighash_type)?,
        ];
        let defaults = [empty_arr(), empty_arr(), null()];
        self.send_request("signrawtransaction", handle_defaults(&mut args, &defaults))
            .await
    }

    pub async fn sign_raw_transaction_with_wallet<R: RawTx>(
        &self,
        tx: R,
        utxos: Option<&[SignRawTransactionInput]>,
        sighash_type: Option<EcdsaSighashType>,
    ) -> Result<SignRawTransactionResult, Error>
    where
        R: Sync + Send,
    {
        let mut args = [
            tx.raw_hex().into(),
            opt_into_json(utxos)?,
            opt_into_json(sighash_type)?,
        ];
        let defaults = [empty_arr(), null()];
        self.send_request(
            "signrawtransactionwithwallet",
            handle_defaults(&mut args, &defaults),
        )
        .await
    }

    pub async fn sign_raw_transaction_with_key<R: RawTx>(
        &self,
        tx: R,
        privkeys: &[PrivateKey],
        prevtxs: Option<&[SignRawTransactionInput]>,
        sighash_type: Option<EcdsaSighashType>,
    ) -> Result<SignRawTransactionResult, Error>
    where
        R: Sync + Send,
    {
        let mut args = [
            tx.raw_hex().into(),
            into_json(privkeys)?,
            opt_into_json(prevtxs)?,
            opt_into_json(sighash_type)?,
        ];
        let defaults = [empty_arr(), null()];
        self.send_request(
            "signrawtransactionwithkey",
            handle_defaults(&mut args, &defaults),
        )
        .await
    }

    pub async fn test_mempool_accept<R: RawTx>(
        &self,
        rawtxs: &[R],
    ) -> Result<Vec<TestMempoolAcceptResult>, Error>
    where
        R: Sync + Send,
    {
        let hexes: Vec<serde_json::Value> =
            rawtxs.iter().cloned().map(|r| r.raw_hex().into()).collect();
        self.send_request("testmempoolaccept", &[hexes.into()])
            .await
    }

    pub async fn stop(&self) -> Result<String, Error> {
        self.send_request("stop", &[]).await
    }

    pub async fn verify_message(
        &self,
        address: &Address,
        signature: &Signature,
        message: &str,
    ) -> Result<bool, Error> {
        let args = [
            address.to_string().into(),
            signature.to_string().into(),
            into_json(message)?,
        ];
        self.send_request("verifymessage", &args).await
    }

    /// Generate new address under own control
    pub async fn get_new_address(
        &self,
        label: Option<&str>,
        address_type: Option<AddressType>,
    ) -> Result<Address, Error> {
        self.send_request(
            "getnewaddress",
            &[opt_into_json(label)?, opt_into_json(address_type)?],
        )
        .await
    }

    pub async fn get_address_info(&self, address: &Address) -> Result<GetAddressInfoResult, Error> {
        self.send_request("getaddressinfo", &[address.to_string().into()])
            .await
    }

    /// Mine `block_num` blocks and pay coinbase to `address`
    ///
    /// Returns hashes of the generated blocks
    pub async fn generate_to_address(
        &self,
        block_num: u64,
        address: &Address,
    ) -> Result<Vec<bitcoin::BlockHash>, Error> {
        self.send_request(
            "generatetoaddress",
            &[block_num.into(), address.to_string().into()],
        )
        .await
    }

    /// Mine up to block_num blocks immediately (before the RPC call returns)
    /// to an address in the wallet.
    pub async fn generate(
        &self,
        block_num: u64,
        maxtries: Option<u64>,
    ) -> Result<Vec<bitcoin::BlockHash>, Error> {
        self.send_request("generate", &[block_num.into(), opt_into_json(maxtries)?])
            .await
    }

    /// Mark a block as invalid by `block_hash`
    pub async fn invalidate_block(&self, block_hash: &bitcoin::BlockHash) -> Result<(), Error> {
        self.send_request("invalidateblock", &[into_json(block_hash)?])
            .await
    }

    /// Mark a block as valid by `block_hash`
    pub async fn reconsider_block(&self, block_hash: &bitcoin::BlockHash) -> Result<(), Error> {
        self.send_request("reconsiderblock", &[into_json(block_hash)?])
            .await
    }

    /// Get txids of all transactions in a memory pool
    pub async fn get_raw_mempool(&self) -> Result<Vec<bitcoin::Txid>, Error> {
        self.send_request("getrawmempool", &[]).await
    }

    /// Get mempool data for given transaction
    pub async fn get_mempool_entry(
        &self,
        txid: &bitcoin::Txid,
    ) -> Result<GetMempoolEntryResult, Error> {
        self.send_request("getmempoolentry", &[into_json(txid)?])
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_to_address(
        &self,
        address: &Address,
        amount: Amount,
        comment: Option<&str>,
        comment_to: Option<&str>,
        subtract_fee: Option<bool>,
        replaceable: Option<bool>,
        confirmation_target: Option<u32>,
        estimate_mode: Option<EstimateMode>,
    ) -> Result<bitcoin::Txid, Error> {
        let mut args = [
            address.to_string().into(),
            into_json(amount.to_btc())?,
            opt_into_json(comment)?,
            opt_into_json(comment_to)?,
            opt_into_json(subtract_fee)?,
            opt_into_json(replaceable)?,
            opt_into_json(confirmation_target)?,
            opt_into_json(estimate_mode)?,
        ];
        self.send_request(
            "sendtoaddress",
            handle_defaults(
                &mut args,
                &[
                    "".into(),
                    "".into(),
                    false.into(),
                    false.into(),
                    6.into(),
                    null(),
                ],
            ),
        )
        .await
    }

    /// Returns data about each connected network node as an array of
    /// [`PeerInfo`][]
    ///
    /// [`PeerInfo`]: net/struct.PeerInfo.html
    pub async fn get_peer_info(&self) -> Result<Vec<GetPeerInfoResult>, Error> {
        self.send_request("getpeerinfo", &[]).await
    }

    /// Requests that a ping be sent to all other nodes, to measure ping
    /// time.
    ///
    /// Results provided in `getpeerinfo`, `pingtime` and `pingwait` fields
    /// are decimal seconds.
    ///
    /// Ping command is handled in queue with all other commands, so it
    /// measures processing backlog, not just network ping.
    pub async fn ping(&self) -> Result<(), Error> {
        self.send_request("ping", &[]).await
    }

    pub async fn send_raw_transaction<R: RawTx>(&self, tx: R) -> Result<bitcoin::Txid, Error>
    where
        R: Sync + Send,
    {
        self.send_request("sendrawtransaction", &[tx.raw_hex().into()])
            .await
    }

    pub async fn estimate_smart_fee(
        &self,
        conf_target: u16,
        estimate_mode: Option<EstimateMode>,
    ) -> Result<EstimateSmartFeeResult, Error> {
        let mut args = [into_json(conf_target)?, opt_into_json(estimate_mode)?];
        self.send_request("estimatesmartfee", handle_defaults(&mut args, &[null()]))
            .await
    }

    /// Waits for a specific new block and returns useful info about it.
    /// Returns the current block on timeout or exit.
    ///
    /// # Arguments
    ///
    /// 1. `timeout`: Time in milliseconds to wait for a response. 0
    /// indicates no timeout.
    pub async fn wait_for_new_block(&self, timeout: u64) -> Result<BlockRef, Error> {
        self.send_request("waitfornewblock", &[into_json(timeout)?])
            .await
    }

    /// Waits for a specific new block and returns useful info about it.
    /// Returns the current block on timeout or exit.
    ///
    /// # Arguments
    ///
    /// 1. `blockhash`: Block hash to wait for.
    /// 2. `timeout`: Time in milliseconds to wait for a response. 0
    /// indicates no timeout.
    pub async fn wait_for_block(
        &self,
        blockhash: &bitcoin::BlockHash,
        timeout: u64,
    ) -> Result<BlockRef, Error> {
        let args = [into_json(blockhash)?, into_json(timeout)?];
        self.send_request("waitforblock", &args).await
    }

    pub async fn wallet_create_funded_psbt(
        &self,
        inputs: &[CreateRawTransactionInput],
        outputs: &HashMap<String, Amount>,
        locktime: Option<i64>,
        options: Option<WalletCreateFundedPsbtOptions>,
        bip32derivs: Option<bool>,
    ) -> Result<WalletCreateFundedPsbtResult, Error> {
        let outputs_converted = serde_json::Map::from_iter(
            outputs
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::from(v.to_btc()))),
        );
        let mut args = [
            into_json(inputs)?,
            into_json(outputs_converted)?,
            opt_into_json(locktime)?,
            opt_into_json(options)?,
            opt_into_json(bip32derivs)?,
        ];
        self.send_request(
            "walletcreatefundedpsbt",
            handle_defaults(
                &mut args,
                &[0.into(), serde_json::Map::new().into(), false.into()],
            ),
        )
        .await
    }

    pub async fn get_descriptor_info(&self, desc: &str) -> Result<GetDescriptorInfoResult, Error> {
        self.send_request("getdescriptorinfo", &[desc.to_string().into()])
            .await
    }

    pub async fn combine_psbt(&self, psbts: &[String]) -> Result<String, Error> {
        self.send_request("combinepsbt", &[into_json(psbts)?]).await
    }

    pub async fn finalize_psbt(
        &self,
        psbt: &str,
        extract: Option<bool>,
    ) -> Result<FinalizePsbtResult, Error> {
        let mut args = [into_json(psbt)?, opt_into_json(extract)?];
        self.send_request("finalizepsbt", handle_defaults(&mut args, &[true.into()]))
            .await
    }

    pub async fn derive_addresses(
        &self,
        descriptor: &str,
        range: Option<[u32; 2]>,
    ) -> Result<Vec<Address>, Error> {
        let mut args = [into_json(descriptor)?, opt_into_json(range)?];
        self.send_request("deriveaddresses", handle_defaults(&mut args, &[null()]))
            .await
    }

    pub async fn rescan_blockchain(
        &self,
        start_from: Option<usize>,
        stop_height: Option<usize>,
    ) -> Result<(usize, Option<usize>), Error> {
        let mut args = [opt_into_json(start_from)?, opt_into_json(stop_height)?];

        #[derive(Deserialize)]
        struct Response {
            pub start_height: usize,
            pub stop_height: Option<usize>,
        }
        let res: Response = self
            .send_request(
                "rescanblockchain",
                handle_defaults(&mut args, &[0.into(), null()]),
            )
            .await?;
        Ok((res.start_height, res.stop_height))
    }

    /// Returns statistics about the unspent transaction output set.
    /// This call may take some time.
    pub async fn get_tx_out_set_info(&self) -> Result<GetTxOutSetInfoResult, Error> {
        self.send_request("gettxoutsetinfo", &[]).await
    }

    /// Returns information about network traffic, including bytes in, bytes out,
    /// and current time.
    pub async fn get_net_totals(&self) -> Result<GetNetTotalsResult, Error> {
        self.send_request("getnettotals", &[]).await
    }

    /// Returns the estimated network hashes per second based on the last n blocks.
    pub async fn get_network_hash_ps(
        &self,
        nblocks: Option<u64>,
        height: Option<u64>,
    ) -> Result<f64, Error> {
        let mut args = [opt_into_json(nblocks)?, opt_into_json(height)?];
        self.send_request(
            "getnetworkhashps",
            handle_defaults(&mut args, &[null(), null()]),
        )
        .await
    }

    /// Returns the total uptime of the server in seconds
    pub async fn uptime(&self) -> Result<u64, Error> {
        self.send_request("uptime", &[]).await
    }

    pub async fn scan_tx_out_set_blocking(
        &self,
        descriptors: &[ScanTxOutRequest],
    ) -> Result<ScanTxOutResult, Error> {
        self.send_request("scantxoutset", &["start".into(), into_json(descriptors)?])
            .await
    }
}
