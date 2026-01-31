use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UploadStatus {
    Stored,
    Updated,
    Skipped,
    Unknown,
}

impl UploadStatus {
    pub(crate) fn log_message(self, file_name: &str) -> String {
        match self {
            UploadStatus::Stored => format!("Stored new mail {}", file_name),
            UploadStatus::Updated => {
                format!("Updated mail {} (differs from existing)", file_name)
            }
            UploadStatus::Skipped => format!("Skipped mail {} (matches existing)", file_name),
            UploadStatus::Unknown => format!("Uploaded {}", file_name),
        }
    }
}

#[derive(Debug)]
pub(crate) struct UploadApiError {
    pub(crate) status: Option<reqwest::StatusCode>,
    pub(crate) retry_after: Option<std::time::Duration>,
    pub(crate) message: String,
}

#[derive(Deserialize)]
struct UploadResponse {
    status: Option<String>,
}

pub(crate) async fn post_file_to_api(
    client: &reqwest::Client,
    api_url: &str,
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<UploadStatus, UploadApiError> {
    let part = Part::bytes(bytes)
        .file_name(file_name.to_string())
        .mime_str("application/octet-stream")
        .map_err(|e| UploadApiError {
            status: None,
            retry_after: None,
            message: format!("failed to build upload payload: {e}"),
        })?;
    let form = Form::new().part("file", part);

    let resp = client
        .post(api_url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| UploadApiError {
            status: None,
            retry_after: None,
            message: format!("failed to send mail to API: {e}"),
        })?;

    let status = resp.status();
    if !status.is_success() {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(std::time::Duration::from_secs);
        let text = resp.text().await.unwrap_or_default();
        return Err(UploadApiError {
            status: Some(status),
            retry_after,
            message: format!("API rejected upload: {} {}", status, text),
        });
    }

    let status = resp
        .json::<UploadResponse>()
        .await
        .ok()
        .and_then(|resp| resp.status)
        .as_deref()
        .map(parse_upload_status)
        .unwrap_or(UploadStatus::Unknown);

    Ok(status)
}

pub(crate) fn upload_backoff(attempts: u32) -> Duration {
    let seconds = 2u64.saturating_pow(attempts.min(10));
    Duration::from_secs(seconds.clamp(2, 300))
}

pub(crate) fn is_retryable_status(status: Option<reqwest::StatusCode>) -> bool {
    status.is_none_or(|s| s == reqwest::StatusCode::TOO_MANY_REQUESTS || s.is_server_error())
}

fn parse_upload_status(status: &str) -> UploadStatus {
    match status.to_ascii_lowercase().as_str() {
        "stored" => UploadStatus::Stored,
        "updated" => UploadStatus::Updated,
        "skipped" => UploadStatus::Skipped,
        _ => UploadStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_upload_status_is_case_insensitive() {
        assert_eq!(parse_upload_status("stored"), UploadStatus::Stored);
        assert_eq!(parse_upload_status("Updated"), UploadStatus::Updated);
        assert_eq!(parse_upload_status("SKIPPED"), UploadStatus::Skipped);
        assert_eq!(parse_upload_status("unknown"), UploadStatus::Unknown);
    }

    #[test]
    fn upload_backoff_clamps_to_bounds() {
        assert_eq!(upload_backoff(0), Duration::from_secs(2));
        assert_eq!(upload_backoff(2), Duration::from_secs(4));
        assert_eq!(upload_backoff(10), Duration::from_secs(300));
        assert_eq!(upload_backoff(20), Duration::from_secs(300));
    }

    #[test]
    fn retryable_statuses_include_rate_limit_and_server_errors() {
        assert!(is_retryable_status(None));
        assert!(is_retryable_status(Some(
            reqwest::StatusCode::TOO_MANY_REQUESTS
        )));
        assert!(is_retryable_status(Some(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        )));
        assert!(!is_retryable_status(Some(reqwest::StatusCode::BAD_REQUEST)));
    }
}
