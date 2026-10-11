use std::time::Duration;

use reqwest::{
    StatusCode,
    multipart::{Form, Part},
};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UploadStatus {
    Stored,
    Skipped,
}

impl UploadStatus {
    pub(crate) fn log_message(self, file_name: &str) -> String {
        match self {
            UploadStatus::Stored => format!("Stored new mail {}", file_name),
            UploadStatus::Skipped => format!("Skipped mail {} (ID already stored)", file_name),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum UploadApiError {
    #[error("upload request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API rejected upload: {status} {message}")]
    Rejected { status: StatusCode, retry_after: Option<Duration>, message: String },

    #[error("invalid upload acknowledgement ({status}): {message}")]
    Protocol { status: StatusCode, message: String },
}

impl UploadApiError {
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            Self::Request(_) | Self::Protocol { .. } => true,
            Self::Rejected { status, .. } => {
                *status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
            }
        }
    }

    pub(crate) fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Rejected { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    pub(crate) fn is_rate_limited(&self) -> bool {
        matches!(self, Self::Rejected { status: StatusCode::TOO_MANY_REQUESTS, .. })
    }
}

#[derive(Deserialize)]
struct UploadResponse {
    status: UploadStatus,
    mail_id: String,
}

pub(crate) async fn post_file_to_api(
    client: &reqwest::Client,
    api_url: &str,
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<UploadStatus, UploadApiError> {
    let part =
        Part::bytes(bytes).file_name(file_name.to_string()).mime_str("application/octet-stream")?;
    let form = Form::new().part("file", part);

    let resp = client.post(api_url).multipart(form).send().await?;

    let status = resp.status();

    if !status.is_success() {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(std::time::Duration::from_secs);

        let message = read_response_body(resp)
            .await
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|error| format!("could not read error response: {error}"));

        return Err(UploadApiError::Rejected { status, retry_after, message });
    }

    let body = read_response_body(resp)
        .await
        .map_err(|error| UploadApiError::Protocol { status, message: error.to_string() })?;

    let response = serde_json::from_slice::<UploadResponse>(&body)
        .map_err(|error| UploadApiError::Protocol { status, message: error.to_string() })?;

    if file_name.strip_prefix("Persistent.Mail.") != Some(response.mail_id.as_str()) {
        return Err(UploadApiError::Protocol {
            status,
            message: format!("mail ID {} does not match {file_name}", response.mail_id),
        });
    }

    if !matches!(
        (status, response.status),
        (StatusCode::CREATED, UploadStatus::Stored) | (StatusCode::OK, UploadStatus::Skipped)
    ) {
        return Err(UploadApiError::Protocol {
            status,
            message: format!("unexpected HTTP status for {:?}", response.status),
        });
    }

    Ok(response.status)
}

async fn read_response_body(mut response: reqwest::Response) -> anyhow::Result<Vec<u8>> {
    let mut body = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        anyhow::ensure!(body.len() + chunk.len() <= 8192, "response body exceeds 8192 bytes");

        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

pub(crate) fn upload_backoff(attempts: u32) -> Duration {
    let seconds = 2u64.saturating_pow(attempts.min(10));

    Duration::from_secs(seconds.clamp(2, 300))
}

#[cfg(test)]
pub(super) mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;
    use crate::watcher::{
        apply_upload_result,
        state::WatcherState,
        store::{FileSig, QueuedUpload, UploadStore},
    };

    pub(in crate::watcher) async fn respond_once(
        response: String,
    ) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind local API");
        let url = format!("http://{}/v2/upload", listener.local_addr().expect("local address"));

        let task = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(5), async move {
                let (mut stream, _) = listener.accept().await.expect("accept upload");
                let request = read_request(&mut stream).await;

                stream.write_all(response.as_bytes()).await.expect("write API response");
                stream.shutdown().await.expect("close response");

                request
            })
            .await
            .expect("local API completed within timeout")
        });

        (url, task)
    }

    pub(in crate::watcher) async fn read_request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0; 4096];

        loop {
            let count = stream.read(&mut buffer).await.expect("read upload");
            assert_ne!(count, 0, "request ended before its body");
            request.extend_from_slice(buffer.get(..count).expect("read fits buffer"));

            if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers =
                    std::str::from_utf8(request.get(..header_end).expect("request headers"))
                        .expect("ASCII request headers");

                let body_len = headers
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .expect("multipart content length")
                    .1
                    .trim()
                    .parse::<usize>()
                    .expect("numeric content length");

                if request.len() >= header_end + 4 + body_len {
                    break;
                }
            }
        }

        request
    }

    pub(in crate::watcher) fn response(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    pub(in crate::watcher) fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("local API client")
    }

    fn pending_upload() -> (WatcherState, QueuedUpload) {
        let mut state = WatcherState::new(UploadStore::default());

        state.enqueue_upload(QueuedUpload {
            path: "Persistent.Mail.123".to_string(),
            sig: Some(FileSig { size: 1, modified: 0 }),
            attempts: 0,
            last_error: None,
            not_before_ms: None,
        });

        let item = state.next_ready_upload(1000).expect("pending upload ready");

        (state, item)
    }

    #[tokio::test]
    async fn upload_acknowledges_stored_mail_and_preserves_multipart_payload() {
        let (url, server) = respond_once(response(
            "201 Created",
            r#"{"status":"stored","mail_id":"123","mail_type":"Battle"}"#,
        ))
        .await;
        let bytes = b"\x00binary mail\xff".to_vec();
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", bytes.clone()).await;

        assert_eq!(result.as_ref().expect("acknowledged upload"), &UploadStatus::Stored);

        let message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);
        assert!(state.uploads.pending.is_empty());
        assert_eq!(message, "Stored new mail Persistent.Mail.123");

        let request = server.await.expect("API task");
        let text = String::from_utf8_lossy(&request);

        assert!(text.starts_with("POST /v2/upload HTTP/1.1\r\n"));
        assert!(text.contains("name=\"file\"; filename=\"Persistent.Mail.123\""));
        assert!(text.contains("Content-Type: application/octet-stream"));
        assert!(request.windows(bytes.len()).any(|part| part == bytes));
    }

    #[tokio::test]
    async fn upload_acknowledges_existing_id_without_claiming_equal_bytes() {
        let (url, server) = respond_once(response(
            "200 OK",
            r#"{"status":"skipped","mail_id":"123","mail_type":"Battle"}"#,
        ))
        .await;
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

        server.await.expect("API task");

        let message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);
        assert_eq!(message, "Skipped mail Persistent.Mail.123 (ID already stored)");
        assert!(state.uploads.pending.is_empty());
    }

    #[tokio::test]
    async fn unverifiable_success_remains_queued_for_retry() {
        for body in [
            "",
            "not json",
            "{}",
            r#"{"status":"stored"}"#,
            r#"{"mail_id":"123"}"#,
            r#"{"status":null,"mail_id":"123"}"#,
            r#"{"status":"skipped","mail_id":123}"#,
            r#"{"status":"updated","mail_id":"123"}"#,
            r#"{"status":"unknown","mail_id":"123"}"#,
            r#"{"status":"stored","mail_id":"456"}"#,
            r#"{"status":"skipped","mail_id":"456"}"#,
            r#"{"status":"skipped","mail_id":"0123"}"#,
        ] {
            let (url, server) = respond_once(response("200 OK", body)).await;
            let (mut state, item) = pending_upload();

            let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

            server.await.expect("API task");
            assert!(
                matches!(&result, Err(UploadApiError::Protocol { .. })),
                "{body:?}: {result:?}"
            );

            let message =
                apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);
            assert!(state.next_ready_upload(2999).is_none());

            let retry =
                state.next_ready_upload(3000).expect("invalid acknowledgement remains pending");
            assert_eq!(retry.path, "Persistent.Mail.123");
            assert_eq!(retry.attempts, 1);
            assert!(message.starts_with("Failed to upload"));
            assert!(state.uploads.api_backoff_until_ms.is_none());
        }
    }

    #[tokio::test]
    async fn truncated_success_body_remains_queued_for_retry() {
        let (url, server) = respond_once(
            "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n{".to_string(),
        )
        .await;
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

        server.await.expect("API task");
        assert!(matches!(&result, Err(UploadApiError::Protocol { .. })));

        let message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);
        assert!(state.next_ready_upload(3000).is_some());
        assert!(message.contains("invalid upload acknowledgement"));
    }

    #[tokio::test]
    async fn upload_rejects_unexpected_http_status_for_known_outcomes() {
        for (http_status, outcome) in
            [("202 Accepted", "stored"), ("200 OK", "stored"), ("201 Created", "skipped")]
        {
            let body = format!(r#"{{"status":"{outcome}","mail_id":"123","mail_type":"Battle"}}"#);
            let (url, server) = respond_once(response(http_status, &body)).await;

            let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

            server.await.expect("API task");
            assert!(
                matches!(result, Err(UploadApiError::Protocol { .. })),
                "{http_status}: {outcome}"
            );
        }
    }

    #[tokio::test]
    async fn connection_failure_remains_queued_for_retry() {
        let (url, server) = respond_once(String::new()).await;
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;
        assert!(matches!(&result, Err(UploadApiError::Request(_))));

        let message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);

        server.await.expect("API task");
        assert!(state.next_ready_upload(2999).is_none());
        assert!(state.next_ready_upload(3000).is_some());
        assert!(message.contains("upload request failed"));
    }

    #[tokio::test]
    async fn server_failure_remains_queued_and_preserves_rejection_reason() {
        let (url, server) = respond_once(response(
            "502 Bad Gateway",
            r#"{"error":"clamav scan failed: unavailable"}"#,
        ))
        .await;
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

        let message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);

        server.await.expect("API task");
        assert!(state.next_ready_upload(3000).is_some());
        assert!(message.contains("clamav scan failed: unavailable"));
        assert!(state.uploads.api_backoff_until_ms.is_none());
    }

    #[tokio::test]
    async fn rate_limit_preserves_pending_item_and_honors_retry_after() {
        let (url, server) = respond_once(
            "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 7\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string(),
        )
        .await;
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

        let message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);

        server.await.expect("API task");
        assert_eq!(state.rate_limit_remaining_ms(1000), Some(7000));
        assert!(state.next_ready_upload(7999).is_none());
        assert!(state.next_ready_upload(8000).is_some());
        assert_eq!(message, "Rate limited by API (429). Pausing uploads for 7s.");
    }

    #[tokio::test]
    async fn zero_retry_after_uses_existing_backoff() {
        let (url, server) = respond_once(
            "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string(),
        )
        .await;
        let (mut state, item) = pending_upload();

        let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

        let _message = apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);

        server.await.expect("API task");
        assert_eq!(state.rate_limit_remaining_ms(1000), Some(2000));
        assert!(state.next_ready_upload(3000).is_some());
    }

    #[tokio::test]
    async fn input_rejections_preserve_reason_without_retry_or_success() {
        for (status, reason) in [
            ("400 Bad Request", "mail id mismatch"),
            ("422 Unprocessable Entity", "unsupported mail type"),
        ] {
            let body = format!(r#"{{"error":"{reason}"}}"#);
            let (url, server) = respond_once(response(status, &body)).await;
            let (mut state, item) = pending_upload();

            let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;
            assert!(matches!(&result, Err(UploadApiError::Rejected { .. })));

            let message =
                apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);

            server.await.expect("API task");
            assert!(state.uploads.pending.is_empty());
            assert!(message.starts_with("Failed to upload"));
            assert!(message.contains(reason));
        }
    }

    #[tokio::test]
    async fn unreadable_rejection_body_preserves_http_failure_classification() {
        for (status, retryable) in [("400 Bad Request", false), ("503 Service Unavailable", true)] {
            let (url, server) = respond_once(format!(
                "HTTP/1.1 {status}\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n{{"
            ))
            .await;
            let (mut state, item) = pending_upload();

            let result = post_file_to_api(&client(), &url, "Persistent.Mail.123", vec![1]).await;

            let message =
                apply_upload_result(&mut state, item, "Persistent.Mail.123", result, 1000);

            server.await.expect("API task");
            assert_eq!(state.next_ready_upload(3000).is_some(), retryable);
            assert!(message.contains("could not read error response"));
        }
    }

    #[test]
    fn upload_backoff_clamps_to_bounds() {
        assert_eq!(upload_backoff(0), Duration::from_secs(2));
        assert_eq!(upload_backoff(2), Duration::from_secs(4));
        assert_eq!(upload_backoff(10), Duration::from_secs(300));
        assert_eq!(upload_backoff(20), Duration::from_secs(300));
    }
}
