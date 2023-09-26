use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

const WEBSERVER_ADDRESS: &str = "WEBSERVER_ADDRESS";
const WEBSERVER_ADDRESS_DEFAULT_PORT: u16 = 3000;
const WEBSERVER_ADDRESS_DEFAULT: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::LOCALHOST,
    WEBSERVER_ADDRESS_DEFAULT_PORT,
));

#[derive(Debug, Clone)]
pub struct Config {
    webserver_address: SocketAddr,
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

        Config { webserver_address }
    }

    pub fn webserver_address(&self) -> SocketAddr {
        self.webserver_address
    }
}
