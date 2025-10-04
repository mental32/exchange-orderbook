use std::net::SocketAddr;

/// The string key used to check the environment variable for the bitcoin grpc bind address.
pub const BITCOIN_GRPC_BIND_ADDR: &str = "BITCOIN_GRPC_BIND_ADDR";

/// Default address value for [`BITCOIN_GRPC_BIND_ADDR`].
pub const BITCOIN_GRPC_BIND_ADDR_DEFAULT: &str = "0.0.0.0:50051";

fn bitcoin_grpc_bind_url_default() -> SocketAddr {
    BITCOIN_GRPC_BIND_ADDR_DEFAULT
        .to_owned()
        .parse()
        .expect("must be able to parse the default address into a SocketAddr")
}

/// The string key used to check the environment variable for the bitcoin rpc url.
pub const BITCOIN_RPC_URL: &str = "BITCOIN_RPC_URL";

/// get the bitcoin rpc url from the environment or panic.
fn bitcoin_rpc_url() -> String {
    std::env::var(BITCOIN_RPC_URL).ok().unwrap_or_else(|| {
        panic!("BITCOIN_RPC_URL env var not set");
    })
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Configuration {
    /// Specifies the address to bind the bitcoin-grpc-proxy socket to
    #[cfg_attr(feature = "serde", serde(default = "bitcoin_grpc_bind_url_default"))]
    pub bitcoin_grpc_bind_addr: SocketAddr,
    #[cfg_attr(feature = "serde", serde(default = "bitcoin_rpc_url"))]
    /// Specifies the URL for the bitcoin-rpc service to connect to
    pub bitcoin_rpc_url: String,
    /// The username for auth
    #[cfg_attr(feature = "serde", serde(default))]
    pub bitcoin_rpc_auth_user: String,
    /// The password for auth
    #[cfg_attr(feature = "serde", serde(default))]
    pub bitcoin_rpc_auth_password: String,
    /// Wallet name for the exchange BTC wallet
    #[cfg_attr(feature = "serde", serde(default))]
    pub bitcoin_wallet_name: String,
}

impl Configuration {
    pub fn from_env(vars: std::env::Vars) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bitcoin_grpc_bind_url_default() {
        assert_eq!(
            bitcoin_grpc_bind_url_default(),
            "0.0.0.0:50051".parse().unwrap()
        );
    }
}
