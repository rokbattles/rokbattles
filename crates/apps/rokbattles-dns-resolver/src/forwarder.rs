//! Bounded DNS-over-HTTPS forwarding for non-target Intra queries.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use hickory_proto::op::{Message, MessageType};
use reqwest::{Client, StatusCode, Url, header, redirect};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{info, warn};

use crate::{DNS_MEDIA_TYPE, MAX_DNS_MESSAGE_BYTES};

const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(3);
const TOTAL_FORWARD_TIMEOUT: Duration = Duration::from_secs(5);
/// Maximum number of DNS queries awaiting a Cloudflare response at once.
pub const MAX_CONCURRENT_UPSTREAM_QUERIES: usize = 256;
/// Primary Cloudflare DNS-over-HTTPS endpoint.
pub const CLOUDFLARE_DOH_PRIMARY_URL: &str = "https://1.1.1.1/dns-query";
/// Fallback Cloudflare DNS-over-HTTPS endpoint.
pub const CLOUDFLARE_DOH_FALLBACK_URL: &str = "https://1.0.0.1/dns-query";

/// A reusable client that forwards DNS wire messages to an upstream DoH resolver.
#[derive(Debug, Clone)]
pub struct DoHForwarder {
    client: Client,
    upstream_urls: Arc<[Url; 2]>,
    primary_healthy: Arc<AtomicBool>,
    upstream_healthy: Arc<AtomicBool>,
    capacity: Arc<Semaphore>,
    capacity_recovery_threshold: usize,
    overload_active: Arc<AtomicBool>,
    total_timeout: Duration,
}

/// Errors encountered while forwarding or validating an upstream DNS response.
#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    /// The HTTP client could not be constructed.
    #[error("failed to build upstream DoH client: {0}")]
    Client(#[source] reqwest::Error),
    /// A built-in endpoint constant could not be parsed.
    #[error("built-in upstream DoH endpoint is invalid")]
    EndpointConfiguration,
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
    /// The fixed forwarding concurrency budget is already in use.
    #[error("upstream DNS forwarding is at capacity")]
    Overloaded,
    /// The primary and fallback attempts exceeded their shared deadline.
    #[error("upstream DNS forwarding exceeded its total deadline")]
    Deadline,
    /// The upstream body was not a response to the forwarded DNS request.
    #[error("upstream returned an invalid or mismatched DNS response")]
    InvalidDnsResponse,
    /// Neither fixed Cloudflare endpoint produced a valid response.
    #[error("both upstream DoH attempts failed (primary: {primary}; fallback: {fallback})")]
    AttemptsFailed { primary: Box<ForwardError>, fallback: Box<ForwardError> },
}

impl DoHForwarder {
    /// Build a Cloudflare forwarding client with a three-second timeout per endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`ForwardError::Client`] if the HTTP client cannot be constructed.
    pub fn new() -> Result<Self, ForwardError> {
        let primary = Url::parse(CLOUDFLARE_DOH_PRIMARY_URL)
            .map_err(|_| ForwardError::EndpointConfiguration)?;
        let fallback = Url::parse(CLOUDFLARE_DOH_FALLBACK_URL)
            .map_err(|_| ForwardError::EndpointConfiguration)?;
        Self::with_options(
            [primary, fallback],
            UPSTREAM_TIMEOUT,
            TOTAL_FORWARD_TIMEOUT,
            MAX_CONCURRENT_UPSTREAM_QUERIES,
            true,
        )
    }

    fn with_options(
        upstream_urls: [Url; 2],
        endpoint_timeout: Duration,
        total_timeout: Duration,
        max_concurrent_queries: usize,
        https_only: bool,
    ) -> Result<Self, ForwardError> {
        let mut client = Client::builder()
            .timeout(endpoint_timeout)
            .no_proxy()
            .redirect(redirect::Policy::none());
        if https_only {
            client = client.https_only(true);
        }
        let client = client.build().map_err(ForwardError::Client)?;
        Ok(Self {
            client,
            upstream_urls: Arc::new(upstream_urls),
            primary_healthy: Arc::new(AtomicBool::new(true)),
            upstream_healthy: Arc::new(AtomicBool::new(true)),
            capacity: Arc::new(Semaphore::new(max_concurrent_queries)),
            capacity_recovery_threshold: max_concurrent_queries.saturating_add(1) / 2,
            overload_active: Arc::new(AtomicBool::new(false)),
            total_timeout,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_test_limits(
        upstream_urls: [Url; 2],
        endpoint_timeout: Duration,
        total_timeout: Duration,
        max_concurrent_queries: usize,
    ) -> Result<Self, ForwardError> {
        Self::with_options(
            upstream_urls,
            endpoint_timeout,
            total_timeout,
            max_concurrent_queries,
            false,
        )
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
        let _permit = self.acquire_capacity()?;
        let result = tokio::time::timeout(
            self.total_timeout,
            self.forward_with_failover(&request, wire_request),
        )
        .await
        .unwrap_or(Err(ForwardError::Deadline));
        self.record_upstream_result(&result);
        result
    }

    fn acquire_capacity(&self) -> Result<OwnedSemaphorePermit, ForwardError> {
        if self.overload_active.load(Ordering::Relaxed)
            && self.capacity.available_permits() >= self.capacity_recovery_threshold
            && self.overload_active.swap(false, Ordering::Relaxed)
        {
            info!("upstream DNS forwarding capacity recovered");
        }
        Arc::clone(&self.capacity).try_acquire_owned().map_err(|_| {
            if !self.overload_active.swap(true, Ordering::Relaxed) {
                warn!("upstream DNS forwarding is at capacity; shedding query");
            }
            ForwardError::Overloaded
        })
    }

    fn record_upstream_result(&self, result: &Result<Vec<u8>, ForwardError>) {
        match result {
            Ok(_) => {
                if !self.upstream_healthy.swap(true, Ordering::Relaxed) {
                    info!("upstream DNS forwarding recovered");
                }
            }
            Err(error) => {
                if self.upstream_healthy.swap(false, Ordering::Relaxed) {
                    warn!(%error, "upstream DNS forwarding is unavailable");
                }
            }
        }
    }

    async fn forward_with_failover(
        &self,
        request: &Message,
        wire_request: &[u8],
    ) -> Result<Vec<u8>, ForwardError> {
        match self.forward_to(request, wire_request, &self.upstream_urls[0]).await {
            Ok(response) => {
                if !self.primary_healthy.swap(true, Ordering::Relaxed) {
                    info!("primary upstream DoH endpoint recovered");
                }
                Ok(response)
            }
            Err(primary) => {
                if self.primary_healthy.swap(false, Ordering::Relaxed) {
                    warn!(error = %primary, "primary upstream DoH query failed; trying fallback");
                }
                match self.forward_to(request, wire_request, &self.upstream_urls[1]).await {
                    Ok(response) => Ok(response),
                    Err(fallback) => Err(ForwardError::AttemptsFailed {
                        primary: Box::new(primary),
                        fallback: Box::new(fallback),
                    }),
                }
            }
        }
    }

    async fn forward_to(
        &self,
        request: &Message,
        wire_request: &[u8],
        upstream_url: &Url,
    ) -> Result<Vec<u8>, ForwardError> {
        let mut response = self
            .client
            .post(upstream_url.clone())
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

        validate_response(request, &wire_response)?;
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
