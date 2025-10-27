use ahash::HashMap;
use std::env::Vars;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;

const BIND_ADDRESS_DEFAULT_PORT: u16 = 7777;

const BIND_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::UNSPECIFIED,
    BIND_ADDRESS_DEFAULT_PORT,
));

fn bind_address() -> SocketAddr {
    BIND_ADDRESS_DEFAULT
}

const DATABASE_URL: &str = "DATABASE_URL";

#[track_caller]
fn database_url() -> String {
    std::env::var(DATABASE_URL)
        .ok()
        .expect("DATABASE_URL env var not set")
}

fn default_bitcoin_grpc_endpoint() -> tonic::transport::Endpoint {
    tonic::transport::Endpoint::from_static("http://[::1]:50051")
}

const BITCOIN_GRPC_ENDPOINT: &str = "BITCOIN_GRPC_ENDPOINT";

const BIND_ADDRESS: &str = "BIND_ADDRESS";

#[cfg(feature = "serde")]
mod endpoint_serde {
    pub fn deserialize<'de, D>(deserializer: D) -> Result<tonic::transport::Endpoint, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize as _;

        let st = String::deserialize(deserializer)?;
        tonic::transport::Endpoint::from_shared(st).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S>(
        endpoint: &tonic::transport::Endpoint,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&endpoint.uri().to_string())
    }
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
        serde(with = "endpoint_serde", default = "default_bitcoin_grpc_endpoint")
    )]
    pub bitcoin_grpc_endpoint: tonic::transport::Endpoint,
    /// the address to bind the webserver
    #[cfg_attr(feature = "serde", serde(default = "bind_address"))]
    pub bind_address: SocketAddr,
}

impl Configuration {
    pub fn from_map(vars: HashMap<String, String>) -> std::io::Result<Self> {
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
        let bind_address = if let Some(addr_str) = vars.get(BIND_ADDRESS) {
            addr_str.parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to parse {}: {}", BIND_ADDRESS, e),
                )
            })?
        } else {
            bind_address()
        };

        Ok(Self {
            database_url,
            bitcoin_grpc_endpoint,
            bind_address,
        })
    }
}
