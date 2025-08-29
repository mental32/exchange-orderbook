use std::net::IpAddr;

use axum::http::HeaderMap;

pub fn rightmost_ip_address(headers: &HeaderMap) -> Option<IpAddr> {
    let rightmost = headers.get_all("X-Forwarded-For").iter().last().cloned()?;
    rightmost.to_str().ok()?.split(",").last()?.parse().ok()
}
