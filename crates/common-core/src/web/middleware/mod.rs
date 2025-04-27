// pub mod msgpack;
// pub use msgpack::Msgpack;

pub mod auth;
pub use auth::{validate_session_token, validate_session_token_or_redirect};

pub mod ip_address {
    use std::net::IpAddr;

    use axum::http::HeaderMap;

    pub fn rightmost_ip_address(headers: &HeaderMap) -> Option<IpAddr> {
        let rightmost = headers.get_all("X-Forwarded-For").iter().last().cloned()?;
        rightmost.to_str().ok()?.split(",").last()?.parse().ok()
    }
}
