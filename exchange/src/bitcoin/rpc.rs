use std::fmt::Debug;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::sync::Arc;
use std::{error, fmt, io};

use ahash::HashMap;
use async_trait::async_trait;
use bitcoin::address::NetworkUnchecked;
use bitcoin::consensus::encode;
use bitcoin::hashes::hex::FromHex;
use bitcoin::hashes::{hex, sha256};
use bitcoin::secp256k1::ecdsa::Signature;
use bitcoin::sighash::EcdsaSighashType;
use bitcoin::{
    secp256k1, Amount, OutPoint, PrivateKey, PublicKey, Script, ScriptBuf, SignedAmount,
    Transaction,
};
use jsonrpc_async;
use rustc_hex::ToHex;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct Address(bitcoin::Address<NetworkUnchecked>);

impl From<bitcoin::Address<NetworkUnchecked>> for Address {
    fn from(addr: bitcoin::Address<NetworkUnchecked>) -> Address {
        Address(addr)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.clone().assume_checked(), f)
    }
}

/// The error type for errors produced in this library.
#[derive(Debug)]
pub enum Error {
    JsonRpc(jsonrpc_async::error::Error),
    Hex(hex::Error),
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

impl From<hex::Error> for Error {
    fn from(e: hex::Error) -> Error {
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
            Error::Hex(ref e) => Some(e),
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
        bitcoin::consensus::encode::serialize(self).to_hex()
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

    /// Query an object implementing `Querable` type
    pub async fn get_by_id<T: Queryable>(&self, id: &<T as Queryable>::Id) -> Result<T, Error>
    where
        T: Sync + Send,
        <T as Queryable>::Id: Sync + Send,
    {
        T::query(self, id).await
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

//TODO(stevenroose) consider using a Time type

/// A module used for serde serialization of bytes in hexadecimal format.
///
/// The module is compatible with the serde attribute.
pub mod serde_hex {
    use bitcoin::hashes::hex::FromHex;
    use rustc_hex::ToHex;
    use serde::de::Error;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(b: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&b.to_hex::<String>())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let hex_str: String = ::serde::Deserialize::deserialize(d)?;
        FromHex::from_hex(&hex_str).map_err(D::Error::custom)
    }

    pub mod opt {
        use bitcoin::hashes::hex::FromHex;
        use rustc_hex::ToHex;
        use serde::de::Error;
        use serde::{Deserializer, Serializer};

        pub fn serialize<S: Serializer>(b: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
            match *b {
                None => s.serialize_none(),
                Some(ref b) => s.serialize_str(&b.to_hex::<String>()),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
            let hex_str: String = ::serde::Deserialize::deserialize(d)?;
            Ok(Some(FromHex::from_hex(&hex_str).map_err(D::Error::custom)?))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetNetworkInfoResultNetwork {
    pub name: String,
    pub limited: bool,
    pub reachable: bool,
    pub proxy: String,
    pub proxy_randomize_credentials: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetNetworkInfoResultAddress {
    pub address: String,
    pub port: usize,
    pub score: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetNetworkInfoResult {
    pub version: usize,
    pub subversion: String,
    #[serde(rename = "protocolversion")]
    pub protocol_version: usize,
    #[serde(rename = "localservices")]
    pub local_services: String,
    #[serde(rename = "localrelay")]
    pub local_relay: bool,
    #[serde(rename = "timeoffset")]
    pub time_offset: isize,
    pub connections: usize,
    #[serde(rename = "networkactive")]
    pub network_active: bool,
    pub networks: Vec<GetNetworkInfoResultNetwork>,
    #[serde(rename = "relayfee", with = "bitcoin::amount::serde::as_btc")]
    pub relay_fee: Amount,
    #[serde(rename = "incrementalfee", with = "bitcoin::amount::serde::as_btc")]
    pub incremental_fee: Amount,
    #[serde(rename = "localaddresses")]
    pub local_addresses: Vec<GetNetworkInfoResultAddress>,
    pub warnings: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMultiSigAddressResult {
    pub address: Address,
    pub redeem_script: ScriptBuf,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct LoadWalletResult {
    pub name: String,
    pub warning: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetWalletInfoResult {
    #[serde(rename = "walletname")]
    pub wallet_name: String,
    #[serde(rename = "walletversion")]
    pub wallet_version: u32,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub balance: Amount,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub unconfirmed_balance: Amount,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub immature_balance: Amount,
    #[serde(rename = "txcount")]
    pub tx_count: usize,
    #[serde(rename = "keypoololdest")]
    pub keypool_oldest: usize,
    #[serde(rename = "keypoolsize")]
    pub keypool_size: usize,
    #[serde(rename = "keypoolsize_hd_internal")]
    pub keypool_size_hd_internal: usize,
    pub unlocked_until: Option<u64>,
    #[serde(rename = "paytxfee", with = "bitcoin::amount::serde::as_btc")]
    pub pay_tx_fee: Amount,
    #[serde(rename = "hdseedid")]
    pub hd_seed_id: Option<bitcoin::hash_types::XpubIdentifier>,
    pub private_keys_enabled: bool,
    pub avoid_reuse: Option<bool>,
    pub scanning: Option<ScanningDetails>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ScanningDetails {
    Scanning {
        duration: usize,
        progress: f32,
    },
    /// The bool in this field will always be false.
    NotScanning(bool),
}

impl Eq for ScanningDetails {}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockResult {
    pub hash: bitcoin::BlockHash,
    pub confirmations: u32,
    pub size: usize,
    pub strippedsize: Option<usize>,
    pub weight: usize,
    pub height: usize,
    pub version: i32,
    #[serde(default, with = "serde_hex::opt")]
    pub version_hex: Option<Vec<u8>>,
    pub merkleroot: bitcoin::hash_types::TxMerkleNode,
    pub tx: Vec<bitcoin::Txid>,
    pub time: usize,
    pub mediantime: Option<usize>,
    pub nonce: u32,
    pub bits: String,
    pub difficulty: f64,
    #[serde(with = "serde_hex")]
    pub chainwork: Vec<u8>,
    pub n_tx: usize,
    pub previousblockhash: Option<bitcoin::BlockHash>,
    pub nextblockhash: Option<bitcoin::BlockHash>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockHeaderResult {
    pub hash: bitcoin::BlockHash,
    pub confirmations: u32,
    pub height: usize,
    pub version: i32,
    #[serde(default, with = "serde_hex::opt")]
    pub version_hex: Option<Vec<u8>>,
    #[serde(rename = "merkleroot")]
    pub merkle_root: bitcoin::hash_types::TxMerkleNode,
    pub time: usize,
    #[serde(rename = "mediantime")]
    pub median_time: Option<usize>,
    pub nonce: u32,
    pub bits: String,
    pub difficulty: f64,
    #[serde(with = "serde_hex")]
    pub chainwork: Vec<u8>,
    pub n_tx: usize,
    #[serde(rename = "previousblockhash")]
    pub previous_block_hash: Option<bitcoin::BlockHash>,
    #[serde(rename = "nextblockhash")]
    pub next_block_hash: Option<bitcoin::BlockHash>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMiningInfoResult {
    pub blocks: u32,
    #[serde(rename = "currentblockweight")]
    pub current_block_weight: Option<u64>,
    #[serde(rename = "currentblocktx")]
    pub current_block_tx: Option<usize>,
    pub difficulty: f64,
    #[serde(rename = "networkhashps")]
    pub network_hash_ps: f64,
    #[serde(rename = "pooledtx")]
    pub pooled_tx: usize,
    pub chain: String,
    pub warnings: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRawTransactionResultVinScriptSig {
    pub asm: String,
    #[serde(with = "serde_hex")]
    pub hex: Vec<u8>,
}

impl GetRawTransactionResultVinScriptSig {
    pub fn script(&self) -> Result<ScriptBuf, encode::Error> {
        Ok(ScriptBuf::from(self.hex.clone()))
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRawTransactionResultVin {
    pub sequence: u32,
    /// The raw scriptSig in case of a coinbase tx.
    #[serde(default, with = "serde_hex::opt")]
    pub coinbase: Option<Vec<u8>>,
    /// Not provided for coinbase txs.
    pub txid: Option<bitcoin::Txid>,
    /// Not provided for coinbase txs.
    pub vout: Option<u32>,
    /// The scriptSig in case of a non-coinbase tx.
    pub script_sig: Option<GetRawTransactionResultVinScriptSig>,
    /// Not provided for coinbase txs.
    #[serde(default, deserialize_with = "deserialize_hex_array_opt")]
    pub txinwitness: Option<Vec<Vec<u8>>>,
}

impl GetRawTransactionResultVin {
    /// Whether this input is from a coinbase tx.
    /// The [txid], [vout] and [script_sig] fields are not provided
    /// for coinbase transactions.
    pub fn is_coinbase(&self) -> bool {
        self.coinbase.is_some()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRawTransactionResultVoutScriptPubKey {
    pub asm: String,
    #[serde(with = "serde_hex")]
    pub hex: Vec<u8>,
    pub req_sigs: Option<usize>,
    #[serde(rename = "type")]
    pub type_: Option<ScriptPubkeyType>,
    pub addresses: Option<Vec<Address>>,
}

impl GetRawTransactionResultVoutScriptPubKey {
    pub fn script(&self) -> Result<ScriptBuf, encode::Error> {
        Ok(ScriptBuf::from(self.hex.clone()))
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRawTransactionResultVout {
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub value: Amount,
    pub n: u32,
    pub script_pub_key: GetRawTransactionResultVoutScriptPubKey,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRawTransactionResult {
    #[serde(rename = "in_active_chain")]
    pub in_active_chain: Option<bool>,
    #[serde(with = "serde_hex")]
    pub hex: Vec<u8>,
    pub txid: bitcoin::Txid,
    pub hash: bitcoin::Wtxid,
    pub size: usize,
    pub vsize: usize,
    pub version: u32,
    pub locktime: u32,
    pub vin: Vec<GetRawTransactionResultVin>,
    pub vout: Vec<GetRawTransactionResultVout>,
    pub blockhash: Option<bitcoin::BlockHash>,
    pub confirmations: Option<u32>,
    pub time: Option<usize>,
    pub blocktime: Option<usize>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetBlockFilterResult {
    pub header: bitcoin::hash_types::FilterHash,
    #[serde(with = "serde_hex")]
    pub filter: Vec<u8>,
}

impl GetBlockFilterResult {
    /// Get the filter.
    /// Note that this copies the underlying filter data. To prevent this,
    /// use [into_filter] instead.
    pub fn to_filter(&self) -> bitcoin::bip158::BlockFilter {
        bitcoin::bip158::BlockFilter::new(&self.filter)
    }

    /// Convert the result in the filter type.
    pub fn into_filter(self) -> bitcoin::bip158::BlockFilter {
        bitcoin::bip158::BlockFilter {
            content: self.filter,
        }
    }
}

impl GetRawTransactionResult {
    /// Whether this tx is a coinbase tx.
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].is_coinbase()
    }

    pub fn transaction(&self) -> Result<Transaction, encode::Error> {
        encode::deserialize(&self.hex)
    }
}

/// Enum to represent the BIP125 replaceable status for a transaction.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Bip125Replaceable {
    Yes,
    No,
    Unknown,
}

/// Enum to represent the category of a transaction.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GetTransactionResultDetailCategory {
    Send,
    Receive,
    Generate,
    Immature,
    Orphan,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct GetTransactionResultDetail {
    pub address: Option<Address>,
    pub category: GetTransactionResultDetailCategory,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub amount: SignedAmount,
    pub label: Option<String>,
    pub vout: u32,
    #[serde(default, with = "bitcoin::amount::serde::as_btc::opt")]
    pub fee: Option<SignedAmount>,
    pub abandoned: Option<bool>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct WalletTxInfo {
    pub confirmations: i32,
    pub blockhash: Option<bitcoin::BlockHash>,
    pub blockindex: Option<usize>,
    pub blocktime: Option<u64>,
    pub blockheight: Option<u32>,
    pub txid: bitcoin::Txid,
    pub time: u64,
    pub timereceived: u64,
    #[serde(rename = "bip125-replaceable")]
    pub bip125_replaceable: Bip125Replaceable,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct GetTransactionResult {
    #[serde(flatten)]
    pub info: WalletTxInfo,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub amount: SignedAmount,
    #[serde(default, with = "bitcoin::amount::serde::as_btc::opt")]
    pub fee: Option<SignedAmount>,
    pub details: Vec<GetTransactionResultDetail>,
    #[serde(with = "serde_hex")]
    pub hex: Vec<u8>,
}

impl GetTransactionResult {
    pub fn transaction(&self) -> Result<Transaction, encode::Error> {
        encode::deserialize(&self.hex)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct ListTransactionResult {
    #[serde(flatten)]
    pub info: WalletTxInfo,
    #[serde(flatten)]
    pub detail: GetTransactionResultDetail,

    pub trusted: Option<bool>,
    pub comment: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct ListSinceBlockResult {
    pub transactions: Vec<ListTransactionResult>,
    #[serde(default)]
    pub removed: Vec<ListTransactionResult>,
    pub lastblock: bitcoin::BlockHash,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTxOutResult {
    pub bestblock: bitcoin::BlockHash,
    pub confirmations: u32,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub value: Amount,
    pub script_pub_key: GetRawTransactionResultVoutScriptPubKey,
    pub coinbase: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListUnspentQueryOptions {
    #[serde(
        rename = "minimumAmount",
        with = "bitcoin::amount::serde::as_btc::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub minimum_amount: Option<Amount>,
    #[serde(
        rename = "maximumAmount",
        with = "bitcoin::amount::serde::as_btc::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub maximum_amount: Option<Amount>,
    #[serde(rename = "maximumCount", skip_serializing_if = "Option::is_none")]
    pub maximum_count: Option<usize>,
    #[serde(
        rename = "minimumSumAmount",
        with = "bitcoin::amount::serde::as_btc::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub minimum_sum_amount: Option<Amount>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUnspentResultEntry {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub address: Option<Address>,
    pub label: Option<String>,
    pub redeem_script: Option<ScriptBuf>,
    pub witness_script: Option<ScriptBuf>,
    pub script_pub_key: ScriptBuf,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub amount: Amount,
    pub confirmations: u32,
    pub spendable: bool,
    pub solvable: bool,
    #[serde(rename = "desc")]
    pub descriptor: Option<String>,
    pub safe: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReceivedByAddressResult {
    #[serde(default, rename = "involvesWatchonly")]
    pub involved_watch_only: bool,
    pub address: Address,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub amount: Amount,
    pub confirmations: u32,
    pub label: String,
    pub txids: Vec<bitcoin::Txid>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignRawTransactionResultError {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub script_sig: ScriptBuf,
    pub sequence: u32,
    pub error: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignRawTransactionResult {
    #[serde(with = "serde_hex")]
    pub hex: Vec<u8>,
    pub complete: bool,
    pub errors: Option<Vec<SignRawTransactionResultError>>,
}

impl SignRawTransactionResult {
    pub fn transaction(&self) -> Result<Transaction, encode::Error> {
        encode::deserialize(&self.hex)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct TestMempoolAcceptResult {
    pub txid: bitcoin::Txid,
    pub allowed: bool,
    #[serde(rename = "reject-reason")]
    pub reject_reason: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bip9SoftforkStatus {
    Defined,
    Started,
    LockedIn,
    Active,
    Failed,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Bip9SoftforkStatistics {
    pub period: u32,
    pub threshold: u32,
    pub elapsed: u32,
    pub count: u32,
    pub possible: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Bip9SoftforkInfo {
    pub status: Bip9SoftforkStatus,
    pub bit: Option<u8>,
    // Can be -1 for 0.18.x inactive ones.
    pub start_time: i64,
    pub timeout: u64,
    pub since: u32,
    pub statistics: Option<Bip9SoftforkStatistics>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SoftforkType {
    Buried,
    Bip9,
}

/// Status of a softfork
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Softfork {
    #[serde(rename = "type")]
    pub type_: SoftforkType,
    pub bip9: Option<Bip9SoftforkInfo>,
    pub height: Option<u32>,
    pub active: bool,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptPubkeyType {
    Nonstandard,
    Pubkey,
    PubkeyHash,
    ScriptHash,
    MultiSig,
    NullData,
    Witness_v0_KeyHash,
    Witness_v0_ScriptHash,
    Witness_Unknown,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetAddressInfoResultEmbedded {
    pub address: Address,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: ScriptBuf,
    #[serde(rename = "is_script")]
    pub is_script: Option<bool>,
    #[serde(rename = "is_witness")]
    pub is_witness: Option<bool>,
    pub witness_version: Option<u32>,
    #[serde(with = "serde_hex")]
    pub witness_program: Vec<u8>,
    pub script: Option<ScriptPubkeyType>,
    /// The redeemscript for the p2sh address.
    #[serde(default, with = "serde_hex::opt")]
    pub hex: Option<Vec<u8>>,
    pub pubkeys: Option<Vec<PublicKey>>,
    #[serde(rename = "sigsrequired")]
    pub n_signatures_required: Option<usize>,
    pub pubkey: Option<PublicKey>,
    #[serde(rename = "is_compressed")]
    pub is_compressed: Option<bool>,
    pub label: Option<String>,
    #[serde(rename = "hdkeypath")]
    pub hd_key_path: Option<bitcoin::bip32::DerivationPath>,
    #[serde(rename = "hdseedid")]
    pub hd_seed_id: Option<bitcoin::hash_types::XpubIdentifier>,
    #[serde(default)]
    pub labels: Vec<GetAddressInfoResultLabel>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GetAddressInfoResultLabelPurpose {
    Send,
    Receive,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetAddressInfoResultLabel {
    Simple(String),
    WithPurpose {
        name: String,
        purpose: GetAddressInfoResultLabelPurpose,
    },
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetAddressInfoResult {
    pub address: Address,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: ScriptBuf,
    #[serde(rename = "ismine")]
    pub is_mine: Option<bool>,
    #[serde(rename = "iswatchonly")]
    pub is_watchonly: Option<bool>,
    #[serde(rename = "isscript")]
    pub is_script: Option<bool>,
    #[serde(rename = "iswitness")]
    pub is_witness: Option<bool>,
    pub witness_version: Option<u32>,
    #[serde(default, with = "serde_hex::opt")]
    pub witness_program: Option<Vec<u8>>,
    pub script: Option<ScriptPubkeyType>,
    /// The redeemscript for the p2sh address.
    #[serde(default, with = "serde_hex::opt")]
    pub hex: Option<Vec<u8>>,
    pub pubkeys: Option<Vec<PublicKey>>,
    #[serde(rename = "sigsrequired")]
    pub n_signatures_required: Option<usize>,
    pub pubkey: Option<PublicKey>,
    /// Information about the address embedded in P2SH or P2WSH, if relevant and known.
    pub embedded: Option<GetAddressInfoResultEmbedded>,
    #[serde(rename = "is_compressed")]
    pub is_compressed: Option<bool>,
    pub timestamp: Option<u64>,
    #[serde(rename = "hdkeypath")]
    pub hd_key_path: Option<bitcoin::bip32::DerivationPath>,
    #[serde(rename = "hdseedid")]
    pub hd_seed_id: Option<bitcoin::hash_types::XpubIdentifier>,
    pub labels: Vec<GetAddressInfoResultLabel>,
    /// Deprecated in v0.20.0. See `labels` field instead.
    #[deprecated(note = "since Core v0.20.0")]
    pub label: Option<String>,
}

/// Models the result of "getblockchaininfo"
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetBlockchainInfoResult {
    /// Current network name as defined in BIP70 (main, test, regtest)
    pub chain: String,
    /// The current number of blocks processed in the server
    pub blocks: u64,
    /// The current number of headers we have validated
    pub headers: u64,
    /// The hash of the currently best block
    #[serde(rename = "bestblockhash")]
    pub best_block_hash: bitcoin::BlockHash,
    /// The current difficulty
    pub difficulty: f64,
    /// Median time for the current best block
    #[serde(rename = "mediantime")]
    pub median_time: u64,
    /// Estimate of verification progress [0..1]
    #[serde(rename = "verificationprogress")]
    pub verification_progress: f64,
    /// Estimate of whether this node is in Initial Block Download mode
    #[serde(rename = "initialblockdownload")]
    pub initial_block_download: bool,
    /// Total amount of work in active chain, in hexadecimal
    #[serde(rename = "chainwork", with = "serde_hex")]
    pub chain_work: Vec<u8>,
    /// The estimated size of the block and undo files on disk
    pub size_on_disk: u64,
    /// If the blocks are subject to pruning
    pub pruned: bool,
    /// Lowest-height complete block stored (only present if pruning is enabled)
    #[serde(rename = "pruneheight")]
    pub prune_height: Option<u64>,
    /// Whether automatic pruning is enabled (only present if pruning is enabled)
    pub automatic_pruning: Option<bool>,
    /// The target size used by pruning (only present if automatic pruning is enabled)
    pub prune_target_size: Option<u64>,
    /// Status of softforks in progress
    #[serde(default)]
    pub softforks: HashMap<String, Softfork>,
    /// Any network and blockchain warnings.
    pub warnings: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ImportMultiRequestScriptPubkey<'a> {
    Address(&'a Address),
    Script(&'a Script),
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetMempoolEntryResult {
    /// Virtual transaction size as defined in BIP 141. This is different from actual serialized
    /// size for witness transactions as witness data is discounted.
    #[serde(alias = "size")]
    pub vsize: u64,
    /// Transaction weight as defined in BIP 141. Added in Core v0.19.0.
    pub weight: Option<u64>,
    /// Local time transaction entered pool in seconds since 1 Jan 1970 GMT
    pub time: u64,
    /// Block height when transaction entered pool
    pub height: u64,
    /// Number of in-mempool descendant transactions (including this one)
    #[serde(rename = "descendantcount")]
    pub descendant_count: u64,
    /// Virtual transaction size of in-mempool descendants (including this one)
    #[serde(rename = "descendantsize")]
    pub descendant_size: u64,
    /// Number of in-mempool ancestor transactions (including this one)
    #[serde(rename = "ancestorcount")]
    pub ancestor_count: u64,
    /// Virtual transaction size of in-mempool ancestors (including this one)
    #[serde(rename = "ancestorsize")]
    pub ancestor_size: u64,
    /// Hash of serialized transaction, including witness data
    pub wtxid: bitcoin::Txid,
    /// Fee information
    pub fees: GetMempoolEntryResultFees,
    /// Unconfirmed transactions used as inputs for this transaction
    pub depends: Vec<bitcoin::Txid>,
    /// Unconfirmed transactions spending outputs from this transaction
    #[serde(rename = "spentby")]
    pub spent_by: Vec<bitcoin::Txid>,
    /// Whether this transaction could be replaced due to BIP125 (replace-by-fee)
    #[serde(rename = "bip125-replaceable")]
    pub bip125_replaceable: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetMempoolEntryResultFees {
    /// Transaction fee in BTC
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub base: Amount,
    /// Transaction fee with fee deltas used for mining priority in BTC
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub modified: Amount,
    /// Modified fees (see above) of in-mempool ancestors (including this one) in BTC
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub ancestor: Amount,
    /// Modified fees (see above) of in-mempool descendants (including this one) in BTC
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub descendant: Amount,
}

impl<'a> serde::Serialize for ImportMultiRequestScriptPubkey<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            ImportMultiRequestScriptPubkey::Address(addr) => {
                #[derive(Serialize)]
                struct Tmp<'a> {
                    pub address: &'a Address,
                }
                serde::Serialize::serialize(&Tmp { address: addr }, serializer)
            }
            ImportMultiRequestScriptPubkey::Script(script) => {
                serializer.serialize_str(&script.as_bytes().to_hex::<String>())
            }
        }
    }
}

/// A import request for importmulti.
///
/// Note: unlike in bitcoind, `timestamp` defaults to 0.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize)]
pub struct ImportMultiRequest<'a> {
    pub timestamp: ImportMultiRescanSince,
    /// If using descriptor, do not also provide address/scriptPubKey, scripts, or pubkeys.
    #[serde(rename = "desc", skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<&'a str>,
    #[serde(rename = "scriptPubKey", skip_serializing_if = "Option::is_none")]
    pub script_pubkey: Option<ImportMultiRequestScriptPubkey<'a>>,
    #[serde(rename = "redeemscript", skip_serializing_if = "Option::is_none")]
    pub redeem_script: Option<&'a Script>,
    #[serde(rename = "witnessscript", skip_serializing_if = "Option::is_none")]
    pub witness_script: Option<&'a Script>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub pubkeys: &'a [PublicKey],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub keys: &'a [PrivateKey],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<(usize, usize)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchonly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keypool: Option<bool>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct ImportMultiOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rescan: Option<bool>,
}

#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum ImportMultiRescanSince {
    Now,
    Timestamp(u64),
}

impl serde::Serialize for ImportMultiRescanSince {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            ImportMultiRescanSince::Now => serializer.serialize_str("now"),
            ImportMultiRescanSince::Timestamp(timestamp) => serializer.serialize_u64(timestamp),
        }
    }
}

impl Default for ImportMultiRescanSince {
    fn default() -> Self {
        ImportMultiRescanSince::Timestamp(0)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ImportMultiResultError {
    pub code: i64,
    pub message: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ImportMultiResult {
    pub success: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub error: Option<ImportMultiResultError>,
}

/// Progress toward rejecting pre-softfork blocks
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct RejectStatus {
    /// `true` if threshold reached
    pub status: bool,
}

/// Models the result of "getpeerinfo"
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPeerInfoResult {
    /// Peer index
    pub id: u64,
    /// The IP address and port of the peer
    // TODO: use a type for addr
    pub addr: String,
    /// Bind address of the connection to the peer
    // TODO: use a type for addrbind
    pub addrbind: String,
    /// Local address as reported by the peer
    // TODO: use a type for addrlocal
    pub addrlocal: Option<String>,
    /// The services offered
    // TODO: use a type for services
    pub services: String,
    /// Whether peer has asked us to relay transactions to it
    pub relaytxes: bool,
    /// The time in seconds since epoch (Jan 1 1970 GMT) of the last send
    pub lastsend: u64,
    /// The time in seconds since epoch (Jan 1 1970 GMT) of the last receive
    pub lastrecv: u64,
    /// The total bytes sent
    pub bytessent: u64,
    /// The total bytes received
    pub bytesrecv: u64,
    /// The connection time in seconds since epoch (Jan 1 1970 GMT)
    pub conntime: u64,
    /// The time offset in seconds
    pub timeoffset: i64,
    /// ping time (if available)
    pub pingtime: Option<f64>,
    /// minimum observed ping time (if any at all)
    pub minping: Option<f64>,
    /// ping wait (if non-zero)
    pub pingwait: Option<f64>,
    /// The peer version, such as 70001
    pub version: u64,
    /// The string version
    pub subver: String,
    /// Inbound (true) or Outbound (false)
    pub inbound: bool,
    /// Whether connection was due to `addnode`/`-connect` or if it was an
    /// automatic/inbound connection
    pub addnode: bool,
    /// The starting height (block) of the peer
    pub startingheight: i64,
    /// The ban score
    pub banscore: i64,
    /// The last header we have in common with this peer
    pub synced_headers: i64,
    /// The last block we have in common with this peer
    pub synced_blocks: i64,
    /// The heights of blocks we're currently asking from this peer
    pub inflight: Vec<u64>,
    /// Whether the peer is whitelisted
    pub whitelisted: bool,
    #[serde(
        rename = "minfeefilter",
        default,
        with = "bitcoin::amount::serde::as_btc::opt"
    )]
    pub min_fee_filter: Option<Amount>,
    /// The total bytes sent aggregated by message type
    pub bytessent_per_msg: HashMap<String, u64>,
    /// The total bytes received aggregated by message type
    pub bytesrecv_per_msg: HashMap<String, u64>,
}

/// Models the result of "estimatesmartfee"
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EstimateSmartFeeResult {
    /// Estimate fee rate in BTC/kB.
    #[serde(
        default,
        rename = "feerate",
        skip_serializing_if = "Option::is_none",
        with = "bitcoin::amount::serde::as_btc::opt"
    )]
    pub fee_rate: Option<Amount>,
    /// Errors encountered during processing.
    pub errors: Option<Vec<String>>,
    /// Block number where estimate was found.
    pub blocks: i64,
}

/// Models the result of "waitfornewblock", and "waitforblock"
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct BlockRef {
    pub hash: bitcoin::BlockHash,
    pub height: u64,
}

/// Models the result of "getdescriptorinfo"
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetDescriptorInfoResult {
    pub descriptor: String,
    pub checksum: String,
    #[serde(rename = "isrange")]
    pub is_range: bool,
    #[serde(rename = "issolvable")]
    pub is_solvable: bool,
    #[serde(rename = "hasprivatekeys")]
    pub has_private_keys: bool,
}

/// Models the result of "walletcreatefundedpsbt"
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct WalletCreateFundedPsbtResult {
    pub psbt: String,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub fee: Amount,
    #[serde(rename = "changepos")]
    pub change_position: i32,
}

/// Models the request for "walletcreatefundedpsbt"
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize, Default)]
pub struct WalletCreateFundedPsbtOptions {
    #[serde(rename = "changeAddress", skip_serializing_if = "Option::is_none")]
    pub change_address: Option<Address>,
    #[serde(rename = "changePosition", skip_serializing_if = "Option::is_none")]
    pub change_position: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_type: Option<AddressType>,
    #[serde(rename = "includeWatching", skip_serializing_if = "Option::is_none")]
    pub include_watching: Option<bool>,
    #[serde(rename = "lockUnspents", skip_serializing_if = "Option::is_none")]
    pub lock_unspent: Option<bool>,
    #[serde(
        rename = "feeRate",
        skip_serializing_if = "Option::is_none",
        with = "bitcoin::amount::serde::as_btc::opt"
    )]
    pub fee_rate: Option<Amount>,
    #[serde(
        rename = "subtractFeeFromOutputs",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub subtract_fee_from_outputs: Vec<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaceable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conf_target: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimate_mode: Option<EstimateMode>,
}

/// Models the result of "finalizepsbt"
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct FinalizePsbtResult {
    pub psbt: Option<String>,
    #[serde(default, with = "serde_hex::opt")]
    pub hex: Option<Vec<u8>>,
    pub complete: bool,
}

impl FinalizePsbtResult {
    pub fn transaction(&self) -> Option<Result<Transaction, encode::Error>> {
        self.hex.as_ref().map(|h| encode::deserialize(h))
    }
}

// Custom types for input arguments.

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum EstimateMode {
    Unset,
    Economical,
    Conservative,
}

/// A wrapper around bitcoin::EcdsaSighashType that will be serialized
/// according to what the RPC expects.
pub struct EcdsaSighashTypeLocal(EcdsaSighashType);

impl From<EcdsaSighashType> for EcdsaSighashTypeLocal {
    fn from(sht: EcdsaSighashType) -> EcdsaSighashTypeLocal {
        EcdsaSighashTypeLocal(sht)
    }
}

impl serde::Serialize for EcdsaSighashTypeLocal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self.0 {
            EcdsaSighashType::All => "ALL",
            EcdsaSighashType::None => "NONE",
            EcdsaSighashType::Single => "SINGLE",
            EcdsaSighashType::AllPlusAnyoneCanPay => "ALL|ANYONECANPAY",
            EcdsaSighashType::NonePlusAnyoneCanPay => "NONE|ANYONECANPAY",
            EcdsaSighashType::SinglePlusAnyoneCanPay => "SINGLE|ANYONECANPAY",
        })
    }
}

// Used for createrawtransaction argument.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateRawTransactionInput {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>,
}

#[derive(Serialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FundRawTransactionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_address: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_position: Option<u32>,
    #[serde(rename = "change_type", skip_serializing_if = "Option::is_none")]
    pub change_type: Option<AddressType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_watching: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_unspents: Option<bool>,
    #[serde(
        with = "bitcoin::amount::serde::as_btc::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub fee_rate: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtract_fee_from_outputs: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaceable: Option<bool>,
    #[serde(rename = "conf_target", skip_serializing_if = "Option::is_none")]
    pub conf_target: Option<u32>,
    #[serde(rename = "estimate_mode", skip_serializing_if = "Option::is_none")]
    pub estimate_mode: Option<EstimateMode>,
}

#[derive(Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FundRawTransactionResult {
    #[serde(with = "serde_hex")]
    pub hex: Vec<u8>,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub fee: Amount,
    #[serde(rename = "changepos")]
    pub change_position: i32,
}

#[derive(Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct GetBalancesResultEntry {
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub trusted: Amount,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub untrusted_pending: Amount,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub immature: Amount,
}

#[derive(Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesResult {
    pub mine: GetBalancesResultEntry,
    pub watchonly: Option<GetBalancesResultEntry>,
}

impl FundRawTransactionResult {
    pub fn transaction(&self) -> Result<Transaction, encode::Error> {
        encode::deserialize(&self.hex)
    }
}

// Used for signrawtransaction argument.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignRawTransactionInput {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub script_pub_key: ScriptBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redeem_script: Option<ScriptBuf>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "bitcoin::amount::serde::as_btc::opt"
    )]
    pub amount: Option<Amount>,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetTxOutSetInfoResult {
    /// The current block height (index)
    pub height: u64,
    /// The hash of the block at the tip of the chain
    #[serde(rename = "bestblock")]
    pub best_block: bitcoin::BlockHash,
    /// The number of transactions with unspent outputs
    pub transactions: u64,
    /// The number of unspent transaction outputs
    #[serde(rename = "txouts")]
    pub tx_outs: u64,
    /// A meaningless metric for UTXO set size
    pub bogosize: u64,
    /// The serialized hash
    pub hash_serialized_2: sha256::Hash,
    /// The estimated size of the chainstate on disk
    pub disk_size: u64,
    /// The total amount
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub total_amount: Amount,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetNetTotalsResult {
    /// Total bytes received
    #[serde(rename = "totalbytesrecv")]
    pub total_bytes_recv: u64,
    /// Total bytes sent
    #[serde(rename = "totalbytessent")]
    pub total_bytes_sent: u64,
    /// Current UNIX time in milliseconds
    #[serde(rename = "timemillis")]
    pub time_millis: u64,
    /// Upload target statistics
    #[serde(rename = "uploadtarget")]
    pub upload_target: GetNetTotalsResultUploadTarget,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct GetNetTotalsResultUploadTarget {
    /// Length of the measuring timeframe in seconds
    #[serde(rename = "timeframe")]
    pub time_frame: u64,
    /// Target in bytes
    pub target: u64,
    /// True if target is reached
    pub target_reached: bool,
    /// True if serving historical blocks
    pub serve_historical_blocks: bool,
    /// Bytes left in current time cycle
    pub bytes_left_in_cycle: u64,
    /// Seconds left in current time cycle
    pub time_left_in_cycle: u64,
}

/// Used to represent an address type.
#[derive(Copy, Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum AddressType {
    Legacy,
    P2shSegwit,
    Bech32,
}

/// Used to represent arguments that can either be an address or a public key.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum PubKeyOrAddress<'a> {
    Address(&'a Address),
    PubKey(&'a PublicKey),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(untagged)]
/// Start a scan of the UTXO set for an [output descriptor](https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md).
pub enum ScanTxOutRequest {
    /// Scan for a single descriptor
    Single(String),
    /// Scan for a descriptor with xpubs
    Extended {
        /// Descriptor
        desc: String,
        /// Range of the xpub derivations to scan
        range: (u64, u64),
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ScanTxOutResult {
    pub success: Option<bool>,
    #[serde(rename = "txouts")]
    pub tx_outs: Option<u64>,
    pub height: Option<u64>,
    #[serde(rename = "bestblock")]
    pub best_block_hash: Option<bitcoin::BlockHash>,
    pub unspents: Vec<Utxo>,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub total_amount: bitcoin::Amount,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Utxo {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub script_pub_key: bitcoin::ScriptBuf,
    #[serde(rename = "desc")]
    pub descriptor: String,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub amount: bitcoin::Amount,
    pub height: u64,
}

impl<'a> serde::Serialize for PubKeyOrAddress<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            PubKeyOrAddress::Address(a) => serde::Serialize::serialize(a, serializer),
            PubKeyOrAddress::PubKey(k) => serde::Serialize::serialize(k, serializer),
        }
    }
}

// Custom deserializer functions.

/// deserialize_hex_array_opt deserializes a vector of hex-encoded byte arrays.
fn deserialize_hex_array_opt<'de, D>(deserializer: D) -> Result<Option<Vec<Vec<u8>>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    //TODO(stevenroose) Revisit when issue is fixed:
    // https://github.com/serde-rs/serde/issues/723

    let v: Vec<String> = Vec::deserialize(deserializer)?;
    let mut res = Vec::new();
    for h in v.into_iter() {
        res.push(FromHex::from_hex(&h).map_err(D::Error::custom)?);
    }
    Ok(Some(res))
}

/// A type that can be queried from Bitcoin Core.
#[async_trait]
pub trait Queryable: Sized + Send + Sync {
    /// Type of the ID used to query the item.
    type Id;
    /// Query the item using `rpc` and convert to `Self`.
    async fn query(rpc: &BitcoinCoreRpcHttp, id: &Self::Id) -> Result<Self, Error>;
}

#[async_trait]
impl Queryable for bitcoin::blockdata::block::Block {
    type Id = bitcoin::BlockHash;

    async fn query(rpc: &BitcoinCoreRpcHttp, id: &Self::Id) -> Result<Self, Error> {
        let rpc_name = "getblock";
        let hex: String = rpc
            .send_request(rpc_name, &[serde_json::to_value(id)?, 0.into()])
            .await?;
        let bytes: Vec<u8> = bitcoin::hashes::hex::FromHex::from_hex(&hex)?;
        Ok(bitcoin::consensus::encode::deserialize(&bytes)?)
    }
}

#[async_trait]
impl Queryable for bitcoin::blockdata::transaction::Transaction {
    type Id = bitcoin::Txid;

    async fn query(rpc: &BitcoinCoreRpcHttp, id: &Self::Id) -> Result<Self, Error> {
        let rpc_name = "getrawtransaction";
        let hex: String = rpc
            .send_request(rpc_name, &[serde_json::to_value(id)?])
            .await?;
        let bytes: Vec<u8> = bitcoin::hashes::hex::FromHex::from_hex(&hex)?;
        Ok(bitcoin::consensus::encode::deserialize(&bytes)?)
    }
}

#[async_trait]
impl Queryable for Option<GetTxOutResult> {
    type Id = bitcoin::OutPoint;

    async fn query(rpc: &BitcoinCoreRpcHttp, id: &Self::Id) -> Result<Self, Error> {
        rpc.get_tx_out(&id.txid, id.vout, Some(true)).await
    }
}
