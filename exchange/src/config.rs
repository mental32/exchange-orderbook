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

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The string key used to check the environment variable for the webserver address.
pub const WEBSERVER_ADDRESS: &str = "WEBSERVER_ADDRESS";

/// The default webserver address port.
pub const WEBSERVER_ADDRESS_DEFAULT_PORT: u16 = 3000;

/// The default webserver address.
pub const WEBSERVER_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::UNSPECIFIED,
    WEBSERVER_ADDRESS_DEFAULT_PORT,
));

/// get the webserver address from the environment or use the default.
fn webserver_address() -> SocketAddr {
    std::env::var(WEBSERVER_ADDRESS)
        .ok()
        .and_then(|st| {
            st.parse()
                .map_err(|err| {
                    tracing::warn!(?err, "Failed to parse WEBSERVER_ADDRESS env var");
                    err
                })
                .ok()
        })
        .unwrap_or(WEBSERVER_ADDRESS_DEFAULT)
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

/// The string key used to check the environment variable for the config file path.
pub const CONFIG_FILE_PATH: &str = "CONFIG_FILE_PATH";

/// get the config file path from the environment or None.
pub fn config_file_path() -> Option<PathBuf> {
    std::env::var(CONFIG_FILE_PATH).ok().map(PathBuf::from)
}

/// The default trading engine channel capacity.
const fn default_te_channel_capacity() -> usize {
    1024
}

/// The string key used to check the environment variable for the bitcoin rpc url.
pub const BITCOIN_RPC_URL: &str = "BITCOIN_RPC_URL";

/// get the bitcoin rpc url from the environment or panic.
fn bitcoin_rpc_url() -> String {
    std::env::var(BITCOIN_RPC_URL).ok().unwrap_or_else(|| {
        panic!("BITCOIN_RPC_URL env var not set");
    })
}

/// the default bitcoin grpc endpoint.
fn default_bitcoin_grpc_endpoint() -> tonic::transport::Endpoint {
    tonic::transport::Endpoint::from_static("http://[::1]:50051")
}
/// The string key used to check the environment variable for the bitcoin **grpc** url.
pub const BITCOIN_GRPC_ENDPOINT: &str = "BITCOIN_GRPC_ENDPOINT";

/// get the bitcoin grpc url from the environment or panic.
fn bitcoin_grpc_url() -> String {
    std::env::var(BITCOIN_RPC_URL).ok().unwrap_or_else(|| {
        panic!("BITCOIN_RPC_URL env var not set");
    })
}

/// The string key used to check the environment variable for the bitcoin grpc bind address.
pub const BITCOIN_GRPC_BIND_ADDR: &str = "BITCOIN_GRPC_BIND_ADDR";

/// Default address value for [`BITCOIN_GRPC_BIND_ADDR`].
pub const BITCOIN_GRPC_BIND_ADDR_DEFAULT: &str = "0.0.0.0:50051";

fn bitcoin_grpc_bind_url_default() -> SocketAddr {
    BITCOIN_GRPC_BIND_ADDR_DEFAULT.to_owned().parse().unwrap()
}

/// deserialize a grpc endpoint from a string.
fn de_grpc_endpoint<'de, D>(deserializer: D) -> Result<tonic::transport::Endpoint, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let st = String::deserialize(deserializer)?;
    tonic::transport::Endpoint::from_shared(st).map_err(serde::de::Error::custom)
}

/// serialize a grpc endpoint to a string.
fn ser_endpoint_to_string<S>(
    endpoint: &tonic::transport::Endpoint,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&endpoint.uri().to_string())
}

/// The string key used to check the environment variable for the directory that stores jinja templates
pub const JINJA_TEMPLATE_DIR: &str = "JINJA_TEMPLATE_DIR";

/// The string key used to check the environment variable for the `/www` dir that stores all frontend (FE) files
pub const FE_WEB_DIR: &str = "FE_WEB_DIR";

/// application "configuration" loaded from a config file, unspecified values may use the environent variables as fallback.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Configuration {
    /// Specifies the address to bind the webserver socket to
    #[serde(default = "webserver_address")]
    pub webserver_bind_addr: SocketAddr,
    /// Specifies the database url (with credentials) to use
    #[serde(default = "database_url")]
    pub database_url: String,
    /// Configure the message channel capacity of the trading engine
    #[serde(default = "default_te_channel_capacity")]
    pub te_channel_capacity: usize,
    /// Mnemonic for the exchange Ether wallet
    pub eth_wallet_mnemonic: Option<String>,
    #[serde(default = "bitcoin_rpc_url")]
    /// Specifies the URL for the bitcoin-rpc service to connect to
    pub bitcoin_rpc_url: String,
    /// The username for auth
    #[serde(default)]
    pub bitcoin_rpc_auth_user: String,
    /// The password for auth
    #[serde(default)]
    pub bitcoin_rpc_auth_password: String,
    /// Wallet name for the exchange BTC wallet
    #[serde(default)]
    pub bitcoin_wallet_name: String,
    /// Specifies the gRPC URL for the bitcoin-grpc-proxy service
    #[serde(
        deserialize_with = "de_grpc_endpoint",
        serialize_with = "ser_endpoint_to_string",
        default = "default_bitcoin_grpc_endpoint"
    )]
    pub bitcoin_grpc_endpoint: tonic::transport::Endpoint,
    /// Specifies the address to bind the bitcoin-grpc-proxy socket to
    #[serde(default = "bitcoin_grpc_bind_url_default")]
    pub bitcoin_grpc_bind_addr: SocketAddr,
    /// Get the path to the template directory for [`minijinja`] or "$CWD/templates/" if not set.
    pub jinja_template_dir: Option<PathBuf>,
    /// the directory that stores all frontend (FE) files like CSS, HTML fragments, robots.txt, fonts
    pub fe_web_dir: Option<PathBuf>,
}

impl Configuration {
    /// load directly from a string, should only be used for tests.
    #[track_caller]
    pub fn load_from_toml(st: &str) -> Self {
        toml::from_str(st).expect("Failed to parse config file")
    }

    /// read the TOML from the file at `path`
    #[track_caller]
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let config_file_path = path.canonicalize()?;
        let st = std::fs::read_to_string(config_file_path)?;
        Ok(toml::from_str(&st)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?)
    }

    /// A tuple of the user and password for bitcoin-rpc auth
    pub fn bitcoin_rpc_auth(&self) -> (String, String) {
        let user = self.bitcoin_rpc_auth_user.clone();
        let password = self.bitcoin_rpc_auth_password.clone();
        (user, password)
    }

    /// Get the path to the template directory for [`minijinja`] or "$CWD/templates/" if not set.
    pub fn jinja_template_dir(&self) -> PathBuf {
        self.jinja_template_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(std::env::var(JINJA_TEMPLATE_DIR).unwrap()))
    }

    /// the directory that stores all frontend (FE) files like CSS, HTML fragments, robots.txt, fonts
    pub fn fe_web_dir(&self) -> PathBuf {
        self.fe_web_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(std::env::var(FE_WEB_DIR).unwrap()))
    }
}
