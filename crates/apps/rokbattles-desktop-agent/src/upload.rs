use std::time::Duration;

use bytes::Bytes;
use reqwest::{
    Client, StatusCode,
    multipart::{Form, Part},
};
use serde::Deserialize;
use zeroize::Zeroizing;

const MAIL_URL: &str = "https://ingress.rokbattles.com/v2/upload";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stored,
    Rejected,
    Retry(Duration),
}

pub(crate) fn client() -> anyhow::Result<Client> {
    Ok(Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(format!(
            "ROKBattles/{} ({}; {}) BackgroundAgent",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .pool_max_idle_per_host(2)
        .build()?)
}

struct MailBytes(Zeroizing<Vec<u8>>);

impl AsRef<[u8]> for MailBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Deserialize)]
struct Response {
    status: String,
}

pub(crate) async fn send(
    client: &Client,
    name: &str,
    bytes: Zeroizing<Vec<u8>>,
    attempts: u32,
) -> Outcome {
    let retry = Outcome::Retry(backoff(attempts));
    let length = bytes.len() as u64;
    // Bytes retains this wiping owner until HTTP releases all body references.
    let body = reqwest::Body::from(Bytes::from_owner(MailBytes(bytes)));
    let Ok(part) = Part::stream_with_length(body, length)
        .file_name(name.to_string())
        .mime_str("application/octet-stream")
    else {
        return retry;
    };
    let Ok(mut response) =
        client.post(MAIL_URL).multipart(Form::new().part("file", part)).send().await
    else {
        return retry;
    };
    let status = response.status();
    if !status.is_success() {
        return classify_failure(
            status,
            response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok()),
            attempts,
        );
    }
    if response.content_length().is_some_and(|length| length > 4096) {
        return retry;
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if body.len() + chunk.len() <= 4096 => body.extend_from_slice(&chunk),
            Ok(Some(_)) | Err(_) => return retry,
            Ok(None) => break,
        }
    }
    match serde_json::from_slice::<Response>(&body) {
        Ok(response) if matches!(response.status.as_str(), "stored" | "updated" | "skipped") => {
            Outcome::Stored
        }
        _ => retry,
    }
}

fn classify_failure(status: StatusCode, retry_after: Option<&str>, attempts: u32) -> Outcome {
    if matches!(status.as_u16(), 400 | 413 | 415 | 422) {
        return Outcome::Rejected;
    }
    let delay = retry_after
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(2, 3600)))
        .unwrap_or_else(|| backoff(attempts));
    // Configuration, authentication, rate and transient server failures retain
    // the durable pending row. No response body is logged or echoed.
    Outcome::Retry(delay)
}

fn backoff(attempts: u32) -> Duration {
    Duration::from_secs(2u64.saturating_pow(attempts.min(10)).clamp(2, 300))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_failures_never_become_final_rejections() {
        for status in [
            StatusCode::UNAUTHORIZED,
            StatusCode::NOT_FOUND,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            assert!(matches!(classify_failure(status, None, 0), Outcome::Retry(_)));
        }
        assert_eq!(classify_failure(StatusCode::UNPROCESSABLE_ENTITY, None, 0), Outcome::Rejected);
        assert_eq!(
            classify_failure(StatusCode::TOO_MANY_REQUESTS, Some("99999999999"), 0),
            Outcome::Retry(Duration::from_secs(3600))
        );
    }
}
