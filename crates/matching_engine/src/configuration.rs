//! Use the [`Configuration`] struct to read platform-wide settings for the exchange.
//!
//! NB: the code here makes the settings universally knowable instead of separate
//! structs for each service so `bitcoin-grpc-proxy` settings are readable from
//! the webserver or the trading engine.
//!
//! In this module there are also public constants that come in pairs of:
//! - some `$CONFIG_VALUE` like `WEBSERVER_ADDRESS`
//! - a default value `${CONFIG_VALUE}_DEFAULT` (notice the _default suffix)
//!
//! These values and names directly correspond to fields in the [`Configuration`]
//! struct. The fields are all public the struct is plain-ol-data (POD).
//!

use std::env::Vars;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;

/// The default webserver address port.
pub const TRADING_ADDRESS_DEFAULT_PORT: u16 = 7777;

/// The default webserver address.
pub const TRADING_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::UNSPECIFIED,
    TRADING_ADDRESS_DEFAULT_PORT,
));

fn trading_address() -> SocketAddr {
    TRADING_ADDRESS_DEFAULT
}

/// The string key used to check the environment variable for the database url.
pub const DATABASE_URL: &str = "DATABASE_URL";

/// get the database url from the environment or panic.
#[track_caller]
pub fn database_url() -> String {
    std::env::var(DATABASE_URL).ok().unwrap_or_else(|| {
        panic!("DATABASE_URL env var not set");
    })
}

/// the default bitcoin grpc endpoint.
fn default_bitcoin_grpc_endpoint() -> tonic::transport::Endpoint {
    tonic::transport::Endpoint::from_static("http://[::1]:50051")
}
/// The string key used to check the environment variable for the bitcoin **grpc** url.
pub const BITCOIN_GRPC_ENDPOINT: &str = "BITCOIN_GRPC_ENDPOINT";

/// deserialize a grpc endpoint from a string.
#[cfg(feature = "serde")]
fn de_grpc_endpoint<'de, D>(deserializer: D) -> Result<tonic::transport::Endpoint, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;

    let st = String::deserialize(deserializer)?;
    tonic::transport::Endpoint::from_shared(st).map_err(serde::de::Error::custom)
}

/// serialize a grpc endpoint to a string.
#[cfg(feature = "serde")]
fn ser_endpoint_to_string<S>(
    endpoint: &tonic::transport::Endpoint,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&endpoint.uri().to_string())
}

/// application "configuration" loaded from a config file, unspecified values may use the environent variables as fallback.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Configuration {
    /// Specifies the database url (with credentials) to use
    #[cfg_attr(feature = "serde", serde(default = "database_url"))]
    pub database_url: String,
    /// Mnemonic for the exchange Ether wallet
    pub eth_wallet_mnemonic: Option<String>,
    /// Specifies the gRPC URL for the bitcoin-grpc-proxy service
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "de_grpc_endpoint",
            serialize_with = "ser_endpoint_to_string",
            default = "default_bitcoin_grpc_endpoint"
        )
    )]
    pub bitcoin_grpc_endpoint: tonic::transport::Endpoint,
    /// the address to bind the webserver
    #[cfg_attr(feature = "serde", serde(default = "trading_address"))]
    pub trading_bind_address: SocketAddr,
}

impl Configuration {
    pub fn from_env_vars(vars: Vars) -> std::io::Result<Self> {
        todo!("parse vars")
    }
}
