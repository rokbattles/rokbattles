//! Authenticated, size-bounded delivery of raw mail-entry batches.

use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{info, warn};

const MAX_UPLOAD_ATTEMPTS: usize = 4;
const RETRY_DELAYS: [Duration; MAX_UPLOAD_ATTEMPTS - 1] =
    [Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const RELAY_UPLOAD_URL: &str = "https://ingress.rokbattles.com/v2/relay/upload";
const RELAY_USER_AGENT: &str = concat!("ROKBattles/", env!("CARGO_PKG_VERSION"), " (Relay)");

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MailContext {
    pub(crate) player_id: Option<i64>,
    pub(crate) server_id: Option<i32>,
}

#[derive(Debug)]
pub(crate) struct MailBatch {
    pub(crate) context: MailContext,
    pub(crate) entries: Vec<Bytes>,
}

#[derive(Debug, Clone)]
pub struct MailUploader {
    inner: Arc<MailUploaderInner>,
}

#[derive(Debug)]
struct MailUploaderInner {
    client: reqwest::Client,
    token: String,
    upload_url: String,
    retry_delays: [Duration; MAX_UPLOAD_ATTEMPTS - 1],
}

impl MailUploader {
    /// Create an uploader for the configured ingress endpoint.
    #[must_use]
    pub fn new(token: String) -> Self {
        Self {
            inner: Arc::new(MailUploaderInner {
                client: reqwest::Client::new(),
                token,
                upload_url: RELAY_UPLOAD_URL.to_string(),
                retry_delays: RETRY_DELAYS,
            }),
        }
    }

    pub(crate) async fn upload(&self, batch: MailBatch) {
        let entry_count = batch.entries.len();
        for attempt in 1..=MAX_UPLOAD_ATTEMPTS {
            match self.send(&batch).await {
                Ok(response) => {
                    let rejected = response
                        .results
                        .iter()
                        .filter(|result| result.status == "rejected")
                        .count();
                    if rejected == 0 {
                        info!(entry_count, attempt, "relay mail batch uploaded");
                    } else {
                        warn!(
                            entry_count,
                            rejected, attempt, "ingress rejected part of a relay mail batch"
                        );
                    }
                    return;
                }
                Err(error) if error.is_retryable() && attempt < MAX_UPLOAD_ATTEMPTS => {
                    let delay = retry_delay(&self.inner.retry_delays, attempt);
                    warn!(
                        entry_count,
                        attempt,
                        next_attempt = attempt + 1,
                        backoff_millis = delay.as_millis(),
                        %error,
                        "relay mail batch upload failed; retrying"
                    );
                    sleep(delay).await;
                }
                Err(error) => {
                    warn!(
                        entry_count,
                        attempt,
                        retryable = error.is_retryable(),
                        %error,
                        "relay mail batch upload failed"
                    );
                    return;
                }
            }
        }
    }

    async fn send(&self, batch: &MailBatch) -> Result<BatchResponse, UploadError> {
        let mut form = Form::new();
        if let Some(server_id) = batch.context.server_id {
            form = form.text("server_id", server_id.to_string());
        }
        if let Some(player_id) = batch.context.player_id {
            form = form.text("player_id", player_id.to_string());
        }
        for entry in &batch.entries {
            let length = u64::try_from(entry.len()).map_err(|_error| UploadError::EntryTooLarge)?;
            let part = Part::stream_with_length(entry.clone(), length)
                .mime_str("application/octet-stream")?;
            form = form.part("mail", part);
        }

        let response = self
            .inner
            .client
            .post(&self.inner.upload_url)
            .bearer_auth(&self.inner.token)
            .header(reqwest::header::USER_AGENT, RELAY_USER_AGENT)
            .multipart(form)
            .timeout(UPLOAD_TIMEOUT)
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    }
}

#[derive(Debug, Deserialize)]
struct BatchResponse {
    results: Vec<BatchResult>,
}

#[derive(Debug, Deserialize)]
struct BatchResult {
    status: String,
}

#[derive(Debug, thiserror::Error)]
enum UploadError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("mail entry is too large to upload")]
    EntryTooLarge,
}

impl UploadError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Http(error) => error.status().map_or_else(
                || {
                    error.is_timeout()
                        || error.is_connect()
                        || error.is_request()
                        || error.is_body()
                },
                retryable_status,
            ),
            Self::EntryTooLarge => false,
        }
    }
}

fn retryable_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            reqwest::StatusCode::REQUEST_TIMEOUT
                | reqwest::StatusCode::TOO_EARLY
                | reqwest::StatusCode::TOO_MANY_REQUESTS
        )
}

fn retry_delay(delays: &[Duration], failed_attempt: usize) -> Duration {
    failed_attempt
        .checked_sub(1)
        .and_then(|index| delays.get(index))
        .copied()
        .unwrap_or_else(|| delays.last().copied().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        time::timeout,
    };

    use super::*;

    impl MailUploader {
        fn for_test(upload_url: String) -> Self {
            Self {
                inner: Arc::new(MailUploaderInner {
                    client: reqwest::Client::new(),
                    token: "secret".to_string(),
                    upload_url,
                    retry_delays: [Duration::ZERO; MAX_UPLOAD_ATTEMPTS - 1],
                }),
            }
        }
    }

    #[test]
    fn relay_user_agent_uses_package_version() {
        assert_eq!(RELAY_USER_AGENT, format!("ROKBattles/{} (Relay)", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn relay_upload_url_uses_ingress_service() {
        assert_eq!(RELAY_UPLOAD_URL, "https://ingress.rokbattles.com/v2/relay/upload");
    }

    #[test]
    fn retryable_statuses_include_transient_http_failures() {
        for status in [
            reqwest::StatusCode::REQUEST_TIMEOUT,
            reqwest::StatusCode::TOO_EARLY,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            reqwest::StatusCode::BAD_GATEWAY,
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            reqwest::StatusCode::GATEWAY_TIMEOUT,
        ] {
            assert!(retryable_status(status), "{status} should be retryable");
        }
    }

    #[test]
    fn retryable_statuses_exclude_permanent_client_failures() {
        for status in [
            reqwest::StatusCode::BAD_REQUEST,
            reqwest::StatusCode::UNAUTHORIZED,
            reqwest::StatusCode::FORBIDDEN,
            reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        ] {
            assert!(!retryable_status(status), "{status} should not be retryable");
        }
    }

    #[test]
    fn retry_delays_use_bounded_exponential_backoff() {
        assert_eq!(
            (
                retry_delay(&RETRY_DELAYS, 1),
                retry_delay(&RETRY_DELAYS, 2),
                retry_delay(&RETRY_DELAYS, 3),
                retry_delay(&RETRY_DELAYS, 4),
            ),
            (
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(4),
            )
        );
    }

    #[tokio::test]
    async fn upload_retries_a_transient_server_failure() {
        let listener =
            TcpListener::bind("127.0.0.1:0").await.expect("test server should bind locally");
        let address = listener.local_addr().expect("test server address should be available");
        let server = tokio::spawn(async move {
            for (status, body) in [
                ("503 Service Unavailable", ""),
                ("200 OK", r#"{"results":[{"status":"stored"}]}"#),
            ] {
                let (mut stream, _) =
                    listener.accept().await.expect("test server should accept request");
                read_request(&mut stream).await;
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.expect("response should write");
            }
            2
        });
        let uploader = MailUploader::for_test(format!("http://{address}/v2/relay/upload"));
        let batch = MailBatch {
            context: MailContext { player_id: Some(123), server_id: Some(1_804) },
            entries: vec![Bytes::from_static(b"mail")],
        };

        uploader.upload(batch).await;
        let attempts = timeout(Duration::from_secs(3), server)
            .await
            .expect("retry should complete")
            .expect("test server should not panic");

        assert_eq!(attempts, 2);
    }

    async fn read_request(stream: &mut TcpStream) {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        let header_end = loop {
            let count = stream.read(&mut buffer).await.expect("request should read");
            assert!(count > 0, "request should include headers");
            request.extend_from_slice(&buffer[..count]);
            if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };
        let headers = std::str::from_utf8(&request[..header_end]).expect("headers should be UTF-8");
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("content length should parse"))
            })
            .expect("request should include content length");
        while request.len() - header_end < content_length {
            let count = stream.read(&mut buffer).await.expect("request body should read");
            assert!(count > 0, "request body should be complete");
            request.extend_from_slice(&buffer[..count]);
        }
    }
}
