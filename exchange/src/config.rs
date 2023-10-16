use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use serde::Deserialize;

const WEBSERVER_ADDRESS: &str = "WEBSERVER_ADDRESS";
const WEBSERVER_ADDRESS_DEFAULT_PORT: u16 = 3000;
const WEBSERVER_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::UNSPECIFIED,
    WEBSERVER_ADDRESS_DEFAULT_PORT,
));

const REDIS_HOST: &str = "REDIS_HOST";
const REDIS_HOST_DEFAULT: &str = "127.0.0.1";

const REDIS_PORT: &str = "REDIS_PORT";
const REDIS_PORT_DEFAULT: u16 = 6379;

const DATABASE_URL: &str = "DATABASE_URL";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    webserver_address: SocketAddr,
    redis_url: String,
    redis_host: String,
    redis_port: u16,
    database_url: String,
}

impl Config {
    #[track_caller]
    pub fn load_from_env() -> Self {
        let webserver_address = std::env::var(WEBSERVER_ADDRESS)
            .ok()
            .and_then(|st| {
                st.parse()
                    .map_err(|err| {
                        tracing::warn!(?err, "Failed to parse WEBSERVER_ADDRESS env var");
                        err
                    })
                    .ok()
            })
            .unwrap_or(WEBSERVER_ADDRESS_DEFAULT);

        let redis_host = std::env::var(REDIS_HOST)
            .ok()
            .unwrap_or_else(|| REDIS_HOST_DEFAULT.to_owned());

        let redis_port = std::env::var(REDIS_PORT)
            .ok()
            .and_then(|st| {
                st.parse()
                    .map_err(|err| {
                        tracing::warn!(?err, "Failed to parse REDIS_PORT env var");
                        err
                    })
                    .ok()
            })
            .unwrap_or(REDIS_PORT_DEFAULT);

        let database_url = std::env::var(DATABASE_URL).expect("DATABASE_URL env var is required");

        let redis_url = format!("redis://{}:{}", redis_host, redis_port);

        Config {
            webserver_address,
            redis_url,
            redis_host,
            redis_port,
            database_url,
        }
    }

    pub fn webserver_address(&self) -> SocketAddr {
        self.webserver_address
    }

    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }

    pub fn redis_host(&self) -> &str {
        &self.redis_host
    }

    pub fn redis_port(&self) -> u16 {
        self.redis_port
    }

    pub(crate) fn database_url(&self) -> String {
        self.database_url.clone()
    }
}
