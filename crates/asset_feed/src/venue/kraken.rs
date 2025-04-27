use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KrakenConfig {
    /// Websocket address to connect to.
    pub websocket_address: String,
    /// List of symbols to track.
    pub track_symbols: Vec<String>,
}
