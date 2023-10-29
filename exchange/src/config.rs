use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const WEBSERVER_ADDRESS: &str = "WEBSERVER_ADDRESS";
const WEBSERVER_ADDRESS_DEFAULT_PORT: u16 = 3000;
const WEBSERVER_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::UNSPECIFIED,
    WEBSERVER_ADDRESS_DEFAULT_PORT,
));

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

const REDIS_HOST: &str = "REDIS_HOST";
const REDIS_HOST_DEFAULT: &str = "127.0.0.1";

fn redis_host() -> String {
    std::env::var(REDIS_HOST)
        .ok()
        .unwrap_or_else(|| REDIS_HOST_DEFAULT.to_owned())
}

const REDIS_PORT: &str = "REDIS_PORT";
const REDIS_PORT_DEFAULT: u16 = 6379;

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

const DATABASE_URL: &str = "DATABASE_URL";

#[track_caller]
fn database_url() -> String {
    std::env::var(DATABASE_URL).ok().unwrap_or_else(|| {
        panic!("DATABASE_URL env var not set");
    })
}

const CONFIG_FILE_PATH: &str = "CONFIG_FILE_PATH";

fn config_file_path() -> Option<PathBuf> {
    std::env::var(CONFIG_FILE_PATH).ok().map(PathBuf::from)
}

const fn default_te_channel_capacity() -> usize {
    1024
}

const BITCOIN_RPC_URL: &str = "BITCOIN_RPC_URL";

fn bitcoin_rpc_url() -> String {
    std::env::var(BITCOIN_RPC_URL).ok().unwrap_or_else(|| {
        panic!("BITCOIN_RPC_URL env var not set");
    })
}

fn default_bitcoin_grpc_endpoint() -> tonic::transport::Endpoint {
    tonic::transport::Endpoint::from_static("http://[::1]:50051")
}

fn de_grpc_endpoint<'de, D>(deserializer: D) -> Result<tonic::transport::Endpoint, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let st = String::deserialize(deserializer)?;
    tonic::transport::Endpoint::from_shared(st).map_err(serde::de::Error::custom)
}

fn ser_endpoint_to_string<S>(
    endpoint: &tonic::transport::Endpoint,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&endpoint.uri().to_string())
}

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
        serialize_with = "ser_endpoint_to_string"
    )]
    bitcoin_grpc_endpoint: tonic::transport::Endpoint,
}

impl Config {
    #[track_caller]
    pub fn load_from_toml(st: &str) -> Self {
        toml::from_str(st).expect("Failed to parse config file")
    }

    #[track_caller]
    pub fn load_from_env() -> Self {
        let config_file_path = config_file_path()
            .expect("CONFIG_FILE_PATH env var not set")
            .canonicalize()
            .expect("Failed to canonicalize config file path");
        let st = std::fs::read_to_string(config_file_path).expect("Failed to read config file");
        toml::from_str(&st).expect("Failed to parse config file")
    }

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

    pub fn webserver_address(&self) -> SocketAddr {
        self.webserver_address
    }

    pub fn redis_url(&self) -> String {
        let Self {
            redis_host,
            redis_port,
            ..
        } = self;
        format!("redis://{redis_host}:{redis_port}")
    }

    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn config_file_path(&self) -> Option<&Path> {
        self.config_file_path.as_ref().map(|p| p.as_ref())
    }

    pub fn te_channel_capacity(&self) -> usize {
        self.te_channel_capacity
    }

    pub fn eth_wallet_mnemonic(&self) -> Option<&str> {
        self.eth_wallet_mnemonic.as_deref()
    }

    pub fn bitcoin_rpc_url(&self) -> &str {
        self.bitcoin_rpc_url.as_str()
    }

    pub fn bitcoin_rpc_auth(&self) -> (String, String) {
        let user = self.bitcoin_rpc_auth_user.clone();
        let password = self.bitcoin_rpc_auth_password.clone();
        (user, password)
    }

    pub fn bitcoin_wallet_name(&self) -> &str {
        self.bitcoin_wallet_name.as_str()
    }

    pub fn bitcoin_grpc_bind_addr(&self) -> SocketAddr {
        "0.0.0.0:50051".to_owned().parse().unwrap()
    }

    pub fn bitcoin_grpc_endpoint(&self) -> &tonic::transport::Endpoint {
        &self.bitcoin_grpc_endpoint
    }
}
