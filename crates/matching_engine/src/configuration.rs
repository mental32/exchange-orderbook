use std::env::Vars;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;

use ahash::HashMap;

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

#[track_caller]
pub fn database_url() -> String {
    std::env::var(DATABASE_URL).ok().unwrap_or_else(|| {
        panic!("DATABASE_URL env var not set");
    })
}

fn default_bitcoin_grpc_endpoint() -> tonic::transport::Endpoint {
    tonic::transport::Endpoint::from_static("http://[::1]:50051")
}
/// The string key used to check the environment variable for the bitcoin **grpc** url.
pub const BITCOIN_GRPC_ENDPOINT: &str = "BITCOIN_GRPC_ENDPOINT";

/// The string key used to check the environment variable for the trading server bind address.
pub const TRADING_BIND_ADDRESS: &str = "TRADING_BIND_ADDRESS";

#[cfg(feature = "serde")]
fn de_grpc_endpoint<'de, D>(deserializer: D) -> Result<tonic::transport::Endpoint, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;

    let st = String::deserialize(deserializer)?;
    tonic::transport::Endpoint::from_shared(st).map_err(serde::de::Error::custom)
}

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
        let vars: HashMap<String, String> = vars.collect();

        // database_url is required
        let database_url = vars.get(DATABASE_URL).cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} environment variable is required", DATABASE_URL),
            )
        })?;

        // bitcoin_grpc_endpoint is optional with default
        let bitcoin_grpc_endpoint = if let Some(endpoint_str) = vars.get(BITCOIN_GRPC_ENDPOINT) {
            tonic::transport::Endpoint::from_shared(endpoint_str.clone()).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to parse {}: {}", BITCOIN_GRPC_ENDPOINT, e),
                )
            })?
        } else {
            default_bitcoin_grpc_endpoint()
        };

        // trading_bind_address is optional with default
        let trading_bind_address = if let Some(addr_str) = vars.get(TRADING_BIND_ADDRESS) {
            addr_str.parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to parse {}: {}", TRADING_BIND_ADDRESS, e),
                )
            })?
        } else {
            trading_address()
        };

        Ok(Self {
            database_url,
            bitcoin_grpc_endpoint,
            trading_bind_address,
        })
    }
}
