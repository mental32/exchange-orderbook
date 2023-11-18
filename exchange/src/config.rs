//! The config for the exchange.
//!
//! The exchange is configured using a config file. The config file is a toml file that contains the following fields:
//!
//! - `webserver_address` - the address to bind the webserver to
//! - `redis_host` - the redis host to connect to
//! - `redis_port` - the redis port to connect to
//! - `database_url` - the database url to connect to
//! - `config_file_path` - the path to the config file
//! - `te_channel_capacity` - the trading engine channel capacity
//! - `eth_wallet_mnemonic` - the ethereum wallet mnemonic
//! - `bitcoin_rpc_url` - the bitcoin rpc url
//! - `bitcoin_rpc_auth_user` - the bitcoin rpc auth user
//! - `bitcoin_rpc_auth_password` - the bitcoin rpc auth password
//! - `bitcoin_wallet_name` - the bitcoin wallet name
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

/// The string key used to check the environment variable for the redis host.
pub const REDIS_HOST: &str = "REDIS_HOST";

/// The default redis host.
pub const REDIS_HOST_DEFAULT: &str = "127.0.0.1";

/// get the redis host from the environment or use the default.
fn redis_host() -> String {
    std::env::var(REDIS_HOST)
        .ok()
        .unwrap_or_else(|| REDIS_HOST_DEFAULT.to_owned())
}

/// The string key used to check the environment variable for the redis port.
pub const REDIS_PORT: &str = "REDIS_PORT";

/// The default redis port.
pub const REDIS_PORT_DEFAULT: u16 = 6379;

/// get the redis port from the environment or use the default.
fn redis_port() -> u16 {
    std::env::var(REDIS_PORT)
        .ok()
        .and_then(|st| {
            st.parse()
                .map_err(|err| {
                    tracing::warn!(?err, "Failed to parse REDIS_PORT env var");
                    err
                })
                .ok()
        })
        .unwrap_or(REDIS_PORT_DEFAULT)
}

/// The string key used to check the environment variable for the database url.
pub const DATABASE_URL: &str = "DATABASE_URL";

/// get the database url from the environment or panic.
#[track_caller]
fn database_url() -> String {
    std::env::var(DATABASE_URL).ok().unwrap_or_else(|| {
        panic!("DATABASE_URL env var not set");
    })
}

/// The string key used to check the environment variable for the config file path.
const CONFIG_FILE_PATH: &str = "CONFIG_FILE_PATH";

/// get the config file path from the environment or None.
fn config_file_path() -> Option<PathBuf> {
    std::env::var(CONFIG_FILE_PATH).ok().map(PathBuf::from)
}

/// The default trading engine channel capacity.
const fn default_te_channel_capacity() -> usize {
    1024
}

/// The string key used to check the environment variable for the bitcoin rpc url.
const BITCOIN_RPC_URL: &str = "BITCOIN_RPC_URL";

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

/// The config for the exchange.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "webserver_address")]
    webserver_address: SocketAddr,
    #[serde(default = "redis_host")]
    redis_host: String,
    #[serde(default = "redis_port")]
    redis_port: u16,
    #[serde(default = "database_url")]
    database_url: String,
    #[serde(default = "config_file_path")]
    config_file_path: Option<PathBuf>,
    #[serde(default = "default_te_channel_capacity")]
    te_channel_capacity: usize,
    eth_wallet_mnemonic: Option<String>,
    #[serde(default = "bitcoin_rpc_url")]
    bitcoin_rpc_url: String,
    bitcoin_rpc_auth_user: String,
    bitcoin_rpc_auth_password: String,
    bitcoin_wallet_name: String,
    #[serde(
        deserialize_with = "de_grpc_endpoint",
        serialize_with = "ser_endpoint_to_string",
        default = "default_bitcoin_grpc_endpoint"
    )]
    bitcoin_grpc_endpoint: tonic::transport::Endpoint,
}

impl Config {
    /// Load the config from the given toml string.
    ///
    /// This function is intended for use in tests.
    ///
    #[track_caller]
    pub fn load_from_toml(st: &str) -> Self {
        toml::from_str(st).expect("Failed to parse config file")
    }

    /// Load the config from the given toml file depending on the `CONFIG_FILE_PATH` env var.
    /// If `CONFIG_FILE_PATH` is not set, this function panics.
    #[track_caller]
    pub fn load_from_env() -> Self {
        let config_file_path = config_file_path()
            .expect("CONFIG_FILE_PATH env var not set")
            .canonicalize()
            .expect("Failed to canonicalize config file path");
        let st = std::fs::read_to_string(config_file_path).expect("Failed to read config file");
        toml::from_str(&st).expect("Failed to parse config file")
    }

    /// diff the config with another config.
    pub fn diff(&self, other: &Self) -> toml::map::Map<String, toml::Value> {
        let mut map = toml::map::Map::new();

        macro_rules! diff {
            ($field:ident) => {
                if self.$field != other.$field {
                    map.insert(
                        stringify!($field).to_owned(),
                        toml::Value::try_from(&self.$field).unwrap(),
                    );
                }
            };
            // handle a list of fields
            ($($field:ident),*) => {
                $(diff!($field);)*
            };
        }

        diff!(webserver_address, redis_host, redis_port, database_url);

        map
    }

    /// Get the webserver address to bind to.
    pub fn webserver_address(&self) -> SocketAddr {
        self.webserver_address
    }

    /// Get the redis url to connect to.
    pub fn redis_url(&self) -> String {
        let Self {
            redis_host,
            redis_port,
            ..
        } = self;
        format!("redis://{redis_host}:{redis_port}")
    }

    /// Get the database url to connect to.
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Get the path to the config file.
    pub fn config_file_path(&self) -> Option<&Path> {
        self.config_file_path.as_ref().map(|p| p.as_ref())
    }

    /// Get the trading engine channel capacity.
    pub fn te_channel_capacity(&self) -> usize {
        self.te_channel_capacity
    }

    /// Get the ethereum wallet mnemonic.
    pub fn eth_wallet_mnemonic(&self) -> Option<&str> {
        self.eth_wallet_mnemonic.as_deref()
    }

    /// Get the bitcoin rpc url.
    pub fn bitcoin_rpc_url(&self) -> &str {
        self.bitcoin_rpc_url.as_str()
    }

    /// Get the bitcoin rpc auth.
    pub fn bitcoin_rpc_auth(&self) -> (String, String) {
        let user = self.bitcoin_rpc_auth_user.clone();
        let password = self.bitcoin_rpc_auth_password.clone();
        (user, password)
    }

    /// Get the bitcoin wallet name.
    pub fn bitcoin_wallet_name(&self) -> &str {
        self.bitcoin_wallet_name.as_str()
    }

    /// Get the bitcoin grpc bind address.
    pub fn bitcoin_grpc_bind_addr(&self) -> SocketAddr {
        "0.0.0.0:50051".to_owned().parse().unwrap()
    }

    /// Get the bitcoin grpc endpoint.
    pub fn bitcoin_grpc_endpoint(&self) -> &tonic::transport::Endpoint {
        &self.bitcoin_grpc_endpoint
    }
}
