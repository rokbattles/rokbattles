//! Authenticated reporting for cache-resistant DNS checks.

use std::{net::IpAddr, time::Duration};

use reqwest::{Client, Url, header::HeaderValue};
use serde::Serialize;

const REPORT_TIMEOUT: Duration = Duration::from_secs(2);

/// Client used to record that this resolver observed a DNS check nonce.
#[derive(Debug, Clone)]
pub struct DnsCheckReporter {
    client: Client,
    callback_url: Url,
    secret: HeaderValue,
}

/// Invalid reporter configuration or a failed proof callback.
#[derive(Debug, thiserror::Error)]
pub enum DnsCheckReporterError {
    #[error("invalid DNS check callback URL")]
    InvalidUrl,
    #[error("DNS check callback URL must use HTTPS")]
    InsecureUrl,
    #[error("DNS check secret is not a valid HTTP header value")]
    InvalidSecret,
    #[error("failed to build DNS check HTTP client: {0}")]
    Client(#[source] reqwest::Error),
    #[error("DNS check callback failed: {0}")]
    Request(#[source] reqwest::Error),
}

#[derive(Serialize)]
struct MarkRequest<'a> {
    nonce: &'a str,
}

impl DnsCheckReporter {
    /// Create an authenticated reporter. Plain HTTP is accepted only for a
    /// loopback test endpoint.
    pub fn new(callback_url: &str, secret: &str) -> Result<Self, DnsCheckReporterError> {
        let callback_url =
            Url::parse(callback_url).map_err(|_| DnsCheckReporterError::InvalidUrl)?;
        let loopback = callback_url
            .host_str()
            .and_then(|host| host.parse::<IpAddr>().ok())
            .is_some_and(|address| address.is_loopback());
        if callback_url.scheme() != "https" && !(callback_url.scheme() == "http" && loopback) {
            return Err(DnsCheckReporterError::InsecureUrl);
        }

        let mut secret =
            HeaderValue::from_str(secret).map_err(|_| DnsCheckReporterError::InvalidSecret)?;
        secret.set_sensitive(true);
        let client = Client::builder()
            .timeout(REPORT_TIMEOUT)
            .build()
            .map_err(DnsCheckReporterError::Client)?;
        Ok(Self { client, callback_url, secret })
    }

    pub(crate) async fn report(&self, nonce: &str) -> Result<(), DnsCheckReporterError> {
        self.client
            .post(self.callback_url.clone())
            .bearer_auth(self.secret.to_str().map_err(|_| DnsCheckReporterError::InvalidSecret)?)
            .json(&MarkRequest { nonce })
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(DnsCheckReporterError::Request)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_callback_should_require_https() {
        let error = DnsCheckReporter::new("http://example.com/mark", "secret")
            .expect_err("public HTTP should be rejected");

        assert!(matches!(error, DnsCheckReporterError::InsecureUrl));
    }

    #[test]
    fn loopback_http_callback_should_be_allowed_for_tests() {
        DnsCheckReporter::new("http://127.0.0.1:8001/mark", "secret")
            .expect("loopback callback should be valid");
    }
}
