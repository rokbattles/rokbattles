//! Authenticated, size-bounded delivery of raw mail-entry batches.

use std::{sync::Arc, time::Duration};

use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tracing::{info, warn};

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
    pub(crate) entries: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct MailUploader {
    inner: Arc<MailUploaderInner>,
}

#[derive(Debug)]
struct MailUploaderInner {
    client: reqwest::Client,
    token: String,
}

impl MailUploader {
    /// Create an uploader for the configured ingress endpoint.
    #[must_use]
    pub fn new(token: String) -> Self {
        Self { inner: Arc::new(MailUploaderInner { client: reqwest::Client::new(), token }) }
    }

    pub(crate) async fn upload(&self, batch: MailBatch) {
        let entry_count = batch.entries.len();
        match self.send(batch).await {
            Ok(response) => {
                let rejected =
                    response.results.iter().filter(|result| result.status == "rejected").count();
                if rejected == 0 {
                    info!(entry_count, "relay mail batch uploaded");
                } else {
                    warn!(entry_count, rejected, "ingress rejected part of a relay mail batch");
                }
            }
            Err(error) => {
                warn!(entry_count, %error, "relay mail batch upload failed");
            }
        }
    }

    async fn send(&self, batch: MailBatch) -> Result<BatchResponse, UploadError> {
        let mut form = Form::new();
        if let Some(server_id) = batch.context.server_id {
            form = form.text("server_id", server_id.to_string());
        }
        if let Some(player_id) = batch.context.player_id {
            form = form.text("player_id", player_id.to_string());
        }
        for entry in batch.entries {
            let part = Part::bytes(entry).mime_str("application/octet-stream")?;
            form = form.part("mail", part);
        }

        let response = self
            .inner
            .client
            .post(RELAY_UPLOAD_URL)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_user_agent_uses_package_version() {
        assert_eq!(RELAY_USER_AGENT, format!("ROKBattles/{} (Relay)", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn relay_upload_url_uses_ingress_service() {
        assert_eq!(RELAY_UPLOAD_URL, "https://ingress.rokbattles.com/v2/relay/upload");
    }
}
