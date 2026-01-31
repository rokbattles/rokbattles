//! Rate limit key extractors.

use std::net::IpAddr;

use axum::http::{HeaderMap, Request};
use tower_governor::GovernorError;
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor};

use crate::config::RateLimitKey;

/// Extracts a rate limit key based on the configured strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitKeyExtractor {
    mode: RateLimitKey,
}

impl RateLimitKeyExtractor {
    pub fn new(mode: RateLimitKey) -> Self {
        Self { mode }
    }
}

impl KeyExtractor for RateLimitKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        match self.mode {
            RateLimitKey::Peer => PeerIpKeyExtractor.extract(req),
            RateLimitKey::Cloudflare => {
                if let Some(ip) = cf_connecting_ip(req.headers()) {
                    return Ok(ip);
                }
                PeerIpKeyExtractor.extract(req)
            }
        }
    }
}

fn cf_connecting_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("cf-connecting-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<IpAddr>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn parses_cf_connecting_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.10"));
        assert_eq!(
            cf_connecting_ip(&headers),
            Some("203.0.113.10".parse().unwrap())
        );
    }
}
