//! Bounded DNS-over-HTTPS forwarding for non-target Intra queries.

use std::time::Duration;

use hickory_proto::op::{Message, MessageType};
use reqwest::{Client, StatusCode, Url, header};

use crate::{DNS_MEDIA_TYPE, MAX_DNS_MESSAGE_BYTES};

const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(3);

/// A reusable client that forwards DNS wire messages to an upstream DoH resolver.
#[derive(Debug, Clone)]
pub struct DoHForwarder {
    client: Client,
    upstream_url: Url,
}

/// Errors encountered while forwarding or validating an upstream DNS response.
#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    /// The HTTP client could not be constructed.
    #[error("failed to build upstream DoH client: {0}")]
    Client(#[source] reqwest::Error),
    /// The upstream HTTP request failed or timed out.
    #[error("upstream DoH request failed: {0}")]
    Request(#[source] reqwest::Error),
    /// The upstream returned an unsuccessful HTTP status.
    #[error("upstream DoH returned HTTP {0}")]
    Status(StatusCode),
    /// The upstream response did not identify DNS wire format.
    #[error("upstream DoH response must use application/dns-message")]
    UnsupportedMediaType,
    /// The upstream response exceeded the maximum DNS message size.
    #[error("upstream DNS response exceeds 65535 bytes")]
    MessageTooLarge,
    /// The forwarded request was not a valid DNS wire message.
    #[error("cannot forward an invalid DNS request")]
    InvalidDnsRequest,
    /// The upstream body was not a response to the forwarded DNS request.
    #[error("upstream returned an invalid or mismatched DNS response")]
    InvalidDnsResponse,
}

impl DoHForwarder {
    /// Build a forwarding client with a three-second total request timeout.
    ///
    /// # Errors
    ///
    /// Returns [`ForwardError::Client`] if the HTTP client cannot be constructed.
    pub fn new(upstream_url: Url) -> Result<Self, ForwardError> {
        Self::with_timeout(upstream_url, UPSTREAM_TIMEOUT)
    }

    fn with_timeout(upstream_url: Url, timeout: Duration) -> Result<Self, ForwardError> {
        let client = Client::builder().timeout(timeout).build().map_err(ForwardError::Client)?;
        Ok(Self { client, upstream_url })
    }

    #[cfg(test)]
    pub(crate) fn with_test_timeout(
        upstream_url: Url,
        timeout: Duration,
    ) -> Result<Self, ForwardError> {
        Self::with_timeout(upstream_url, timeout)
    }

    /// Forward one DNS wire request and return the validated wire response.
    ///
    /// # Errors
    ///
    /// Returns [`ForwardError`] when the HTTP exchange fails, the response is too
    /// large, or the upstream response does not match the request.
    pub async fn forward(&self, wire_request: &[u8]) -> Result<Vec<u8>, ForwardError> {
        let request =
            Message::from_vec(wire_request).map_err(|_| ForwardError::InvalidDnsRequest)?;
        let mut response = self
            .client
            .post(self.upstream_url.clone())
            .header(header::ACCEPT, DNS_MEDIA_TYPE)
            .header(header::CONTENT_TYPE, DNS_MEDIA_TYPE)
            .body(wire_request.to_vec())
            .send()
            .await
            .map_err(ForwardError::Request)?;

        if !response.status().is_success() {
            return Err(ForwardError::Status(response.status()));
        }
        if !has_dns_media_type(response.headers()) {
            return Err(ForwardError::UnsupportedMediaType);
        }
        if response.content_length().is_some_and(|length| length > MAX_DNS_MESSAGE_BYTES as u64) {
            return Err(ForwardError::MessageTooLarge);
        }

        let mut wire_response = Vec::with_capacity(
            response
                .content_length()
                .and_then(|length| usize::try_from(length).ok())
                .unwrap_or_default(),
        );
        while let Some(chunk) = response.chunk().await.map_err(ForwardError::Request)? {
            if wire_response.len().saturating_add(chunk.len()) > MAX_DNS_MESSAGE_BYTES {
                return Err(ForwardError::MessageTooLarge);
            }
            wire_response.extend_from_slice(&chunk);
        }

        validate_response(&request, &wire_response)?;
        Ok(wire_response)
    }
}

fn has_dns_media_type(headers: &header::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(DNS_MEDIA_TYPE))
}

fn validate_response(request: &Message, wire_response: &[u8]) -> Result<(), ForwardError> {
    let response =
        Message::from_vec(wire_response).map_err(|_| ForwardError::InvalidDnsResponse)?;
    if response.metadata.message_type != MessageType::Response
        || response.metadata.id != request.metadata.id
        || response.queries != request.queries
    {
        return Err(ForwardError::InvalidDnsResponse);
    }
    Ok(())
}
