//! Authenticated, size-bounded delivery of raw mail-entry batches.

use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;

const UPLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const RELAY_UPLOAD_URL: &str = "https://ingress.rokbattles.com/v2/relay/upload";
const RELAY_USER_AGENT: &str = concat!("ROKBattles/", env!("CARGO_PKG_VERSION"), " (Relay)");

/// Optional server-provided context used by ingress reconstruction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailContext {
    /// Player ID from the login response, if observed.
    pub player_id: Option<i64>,
    /// Kingdom ID from login or a mail entry, if observed.
    pub server_id: Option<i32>,
}

/// One multipart request containing bounded raw MailEntity entries.
#[derive(Debug)]
pub struct MailBatch {
    /// Context shared by these entries.
    pub context: MailContext,
    /// Exact network protobuf entries, not reconstructed persistent files.
    pub entries: Vec<Bytes>,
}

/// Shared HTTPS client for the existing relay ingress contract.
#[derive(Clone)]
pub struct MailUploader {
    inner: Arc<MailUploaderInner>,
}

struct MailUploaderInner {
    client: reqwest::Client,
    token: String,
    upload_url: String,
}

impl MailUploader {
    /// Create an uploader for the configured ingress endpoint.
    ///
    /// # Errors
    /// Returns an error if the HTTPS client cannot initialize.
    pub fn new(token: String) -> Result<Self, reqwest::Error> {
        Ok(Self {
            inner: Arc::new(MailUploaderInner {
                client: reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()?,
                token,
                upload_url: RELAY_UPLOAD_URL.to_string(),
            }),
        })
    }

    /// Send one attempt and return the number of unsupported entries.
    ///
    /// A valid acknowledgement completes the entire batch, including rejected
    /// entries. Callers can discard it without storing or retrying those entries.
    ///
    /// # Errors
    /// Returns transport, HTTP, or malformed acknowledgement errors. Callers
    /// retain the batch for retry when the request fails.
    pub async fn upload_once(&self, batch: &MailBatch) -> Result<usize, UploadError> {
        let response = self.send(batch).await?;
        if response.results.len() != batch.entries.len() {
            return Err(UploadError::Acknowledgement);
        }
        let mut rejected = 0;
        for result in response.results {
            match result.status.as_str() {
                "stored" | "skipped" | "updated" => {}
                "rejected" => rejected += 1,
                _ => return Err(UploadError::Acknowledgement),
            }
        }
        Ok(rejected)
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

        let mut response = self
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
        if !response.status().is_success() {
            return Err(UploadError::Acknowledgement);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len().saturating_add(chunk.len()) > 64 * 1024 {
                return Err(UploadError::Acknowledgement);
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(|_error| UploadError::Acknowledgement)
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

/// Transport or acknowledgement failure from an upload attempt.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("HTTP request failed: {0}")]
    /// Transport or HTTP status failure.
    Http(#[from] reqwest::Error),
    #[error("mail entry is too large to upload")]
    /// Entry length cannot be represented in the upload request.
    EntryTooLarge,
    #[error("ingress returned an invalid batch acknowledgement")]
    /// Response was too large, malformed, or did not account for every entry.
    Acknowledgement,
}

impl std::fmt::Debug for MailUploader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MailUploader")
            .field("upload_url", &self.inner.upload_url)
            .finish_non_exhaustive()
    }
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
                    client: reqwest::Client::builder()
                        .redirect(reqwest::redirect::Policy::none())
                        .build()
                        .expect("test client"),
                    token: "secret".to_string(),
                    upload_url,
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

    #[tokio::test]
    async fn failed_request_can_be_retried_successfully() {
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

        uploader.upload_once(&batch).await.expect_err("first request failed");
        assert_eq!(uploader.upload_once(&batch).await.expect("retry succeeds"), 0);
        let attempts = timeout(Duration::from_secs(3), server)
            .await
            .expect("retry should complete")
            .expect("test server should not panic");

        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn upload_contract_preserves_path_token_context_and_raw_entries() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("request");
            let request = read_request(&mut stream).await;
            let body = r#"{"results":[{"status":"stored"},{"status":"rejected"}]}"#;
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .expect("response");
            request
        });
        let uploader = MailUploader::for_test(format!("http://{address}/v2/relay/upload"));
        let batch = MailBatch {
            context: MailContext { player_id: Some(42), server_id: Some(1804) },
            entries: vec![Bytes::from_static(b"\0first\xff"), Bytes::from_static(b"second")],
        };
        assert_eq!(uploader.upload_once(&batch).await.expect("ack"), 1);
        let request =
            timeout(Duration::from_secs(3), server).await.expect("deadline").expect("server");
        let header_end =
            request.windows(4).position(|part| part == b"\r\n\r\n").expect("header terminator");
        let headers =
            String::from_utf8_lossy(request.get(..header_end).expect("headers")).to_lowercase();
        assert!(headers.starts_with("post /v2/relay/upload http/1.1\r\n"));
        assert!(headers.contains("authorization: bearer secret\r\n"));
        assert!(headers.contains(&format!("user-agent: {}", RELAY_USER_AGENT.to_lowercase())));
        for expected in [
            b"name=\"player_id\"\r\n\r\n42".as_slice(),
            b"name=\"server_id\"\r\n\r\n1804",
            b"\r\n\r\n\0first\xff\r\n",
            b"\r\n\r\nsecond\r\n",
        ] {
            assert!(
                request.windows(expected.len()).any(|part| part == expected),
                "multipart bytes differ"
            );
        }
    }

    #[tokio::test]
    async fn redirects_cannot_forward_mail_or_acknowledge_delivery() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("request");
            read_request(&mut stream).await;
            stream.write_all(b"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:1/elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.expect("redirect");
        });
        let uploader = MailUploader::for_test(format!("http://{address}/v2/relay/upload"));
        let batch = MailBatch {
            context: MailContext::default(),
            entries: vec![Bytes::from_static(b"mail")],
        };
        assert!(matches!(uploader.upload_once(&batch).await, Err(UploadError::Acknowledgement)));
        server.await.expect("server");
    }

    async fn read_request(stream: &mut TcpStream) -> Vec<u8> {
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
        request
    }
}
