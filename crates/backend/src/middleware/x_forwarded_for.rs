use axum::http::HeaderMap;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct XForwardedFor(Arc<[Result<IpAddr, (std::net::AddrParseError, String)>]>);

impl XForwardedFor {
    pub fn from_header_map(header_map: &HeaderMap) -> Option<Self> {
        let ips = header_map
            .get_all("X-Forwarded-For")
            .into_iter()
            .filter_map(|v| v.to_str().ok())
            .map(|v| {
                v.split(',')
                    .map(|ip| ip.trim().parse().map_err(|err| (err, ip.to_owned())))
            })
            .flatten()
            .collect::<Vec<_>>();

        if ips.is_empty() {
            None
        } else {
            Some(XForwardedFor(ips.into()))
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Result<IpAddr, (std::net::AddrParseError, String)>> {
        self.0.iter().cloned()
    }

    pub fn rightmost(&self) -> Option<IpAddr> {
        self.0.last().and_then(|ip| ip.clone().ok())
    }
}
