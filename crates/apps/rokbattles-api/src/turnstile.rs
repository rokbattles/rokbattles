//! Turnstile token verification through Cloudflare's Siteverify API.
//!
//! Tokens must be issued for the program application action and an allowed
//! frontend hostname. Missing configuration and unsuccessful verification both
//! reject the request. Tokens and secrets are never included in diagnostics.

use std::time::Duration;

use serde::Deserialize;

use crate::error::ApiError;

const SITEVERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const VERIFICATION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_TOKEN_BYTES: usize = 2048;

/// Action shared by the program application widget and server verification.
pub const PROGRAM_APPLICATION_ACTION: &str = "program_application";

/// Verifies program application tokens against the configured frontend hostnames.
///
/// Clones share the HTTP client's connection pool.
#[derive(Clone)]
pub struct Turnstile {
    client: reqwest::Client,
    secret: Option<String>,
    hostnames: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Verification {
    success: bool,
    hostname: Option<String>,
    action: Option<String>,
    #[serde(rename = "error-codes", default)]
    error_codes: Vec<String>,
}

impl Turnstile {
    /// Creates a verifier with the given secret and allowed frontend hostnames.
    ///
    /// Hostnames must not include schemes or ports. An absent or blank secret,
    /// or an empty hostname list, causes [`Self::verify`] to reject submissions.
    pub fn new(secret: Option<String>, hostnames: Vec<String>) -> Self {
        Self { client: reqwest::Client::new(), secret, hostnames }
    }

    /// Verifies a token issued for a program application.
    ///
    /// Requires a successful Siteverify response with the expected action and
    /// an allowed hostname. The request times out after ten seconds.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Forbidden`] when configuration is missing, the token
    /// is invalid, expired, or already used, or Siteverify cannot be reached or
    /// returns an unsuccessful or invalid response.
    pub async fn verify(&self, token: &str) -> Result<(), ApiError> {
        self.verify_at(token, SITEVERIFY_URL).await
    }

    async fn verify_at(&self, token: &str, endpoint: &str) -> Result<(), ApiError> {
        let Some(secret) = self.secret.as_deref().filter(|value| !value.trim().is_empty()) else {
            tracing::warn!("Turnstile secret key is not configured");
            return Err(ApiError::Forbidden("verification-unavailable".into()));
        };

        if self.hostnames.is_empty() {
            tracing::warn!("Turnstile hostnames are not configured");
            return Err(ApiError::Forbidden("verification-unavailable".into()));
        }

        if token.is_empty() || token.len() > MAX_TOKEN_BYTES {
            return Err(verification_failed());
        }

        let response = self
            .client
            .post(endpoint)
            .timeout(VERIFICATION_TIMEOUT)
            .form(&[("secret", secret), ("response", token)])
            .send()
            .await
            .map_err(|_error| {
                tracing::warn!("Turnstile verification request failed");
                verification_failed()
            })?
            .error_for_status()
            .map_err(|_error| {
                tracing::warn!("Turnstile verification returned an unsuccessful HTTP status");
                verification_failed()
            })?;

        let verification = response.json::<Verification>().await.map_err(|_error| {
            tracing::warn!("Turnstile verification returned an invalid response");
            verification_failed()
        })?;

        if verification.error_codes.iter().any(|code| {
            matches!(
                code.as_str(),
                "missing-input-secret" | "invalid-input-secret" | "internal-error"
            )
        }) {
            tracing::warn!("Turnstile reported a configuration or upstream service failure");
        }

        if !verification.matches(&self.hostnames) {
            return Err(verification_failed());
        }

        Ok(())
    }
}

impl Verification {
    fn matches(&self, hostnames: &[String]) -> bool {
        self.success
            && self.action.as_deref() == Some(PROGRAM_APPLICATION_ACTION)
            && self.hostname.as_ref().is_some_and(|hostname| hostnames.contains(hostname))
    }
}

fn verification_failed() -> ApiError {
    ApiError::Forbidden("verification-failed".into())
}

#[cfg(test)]
mod tests {
    use axum::{Router, http::StatusCode, routing::post};

    use super::*;

    async fn verifier(
        status: StatusCode,
        body: &'static str,
    ) -> (Turnstile, String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route("/", post(move || async move { (status, body) })),
            )
            .await
            .expect("mock server");
        });
        let verifier = Turnstile::new(Some("test-secret".into()), vec!["localhost".into()]);

        (verifier, endpoint, server)
    }

    #[test]
    fn requires_success_and_exact_hostname_and_action() {
        let hostnames = vec!["rokbattles.com".to_owned()];

        for (success, hostname, action, expected) in [
            (true, Some("rokbattles.com"), Some("program_application"), true),
            (false, Some("rokbattles.com"), Some("program_application"), false),
            (true, Some("localhost"), Some("program_application"), false),
            (true, Some("rokbattles.com.evil.com"), Some("program_application"), false),
            (true, None, Some("program_application"), false),
            (true, Some("rokbattles.com"), None, false),
            (true, Some("rokbattles.com"), Some("login"), false),
        ] {
            let response = Verification {
                success,
                hostname: hostname.map(str::to_owned),
                action: action.map(str::to_owned),
                error_codes: vec![],
            };
            assert_eq!(response.matches(&hostnames), expected);
        }
    }

    #[tokio::test]
    async fn missing_configuration_rejects_without_contacting_cloudflare() {
        for (secret, hostnames) in
            [(None, vec!["localhost".into()]), (Some("secret".into()), vec![])]
        {
            assert!(Turnstile::new(secret, hostnames).verify("token").await.is_err());
        }
    }

    #[tokio::test]
    async fn transport_rejects_non_success_status_malformed_json_and_failed_verification() {
        for (status, body) in [
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"success":true,"hostname":"localhost","action":"program_application"}"#,
            ),
            (StatusCode::OK, "not json"),
            (StatusCode::OK, r#"{"success":false}"#),
            (
                StatusCode::OK,
                r#"{"success":true,"hostname":"evil.com","action":"program_application"}"#,
            ),
            (StatusCode::OK, r#"{"success":true,"hostname":"localhost","action":"other"}"#),
        ] {
            let (verifier, endpoint, server) = verifier(status, body).await;
            assert!(matches!(
                verifier.verify_at("token", &endpoint).await,
                Err(ApiError::Forbidden(_))
            ));

            server.abort();
        }
    }

    #[tokio::test]
    async fn transport_accepts_valid_response_and_rejects_empty_and_oversized_tokens() {
        let (verifier, endpoint, server) = verifier(
            StatusCode::OK,
            r#"{"success":true,"hostname":"localhost","action":"program_application"}"#,
        )
        .await;
        verifier.verify_at("token", &endpoint).await.expect("valid verification");

        for token in ["".to_owned(), "a".repeat(2049)] {
            assert!(verifier.verify_at(&token, &endpoint).await.is_err());
        }

        server.abort();
    }

    #[tokio::test]
    async fn transport_failure_rejects_without_exposing_secrets() {
        let (verifier, endpoint, server) = verifier(StatusCode::OK, "{}").await;
        server.abort();
        let error =
            verifier.verify_at("private-token", &endpoint).await.expect_err("must fail closed");

        assert_eq!(error.to_string(), "verification-failed");
    }

    #[tokio::test]
    async fn stalled_siteverify_request_times_out_and_fails_closed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/",
                    post(|| async {
                        tokio::time::sleep(Duration::from_secs(20)).await;
                        "{}"
                    }),
                ),
            )
            .await
            .expect("mock server");
        });
        let verifier = Turnstile::new(Some("test-secret".into()), vec!["localhost".into()]);
        let started = std::time::Instant::now();
        assert_eq!(
            verifier.verify_at("token", &endpoint).await.expect_err("timeout").to_string(),
            "verification-failed"
        );
        assert!(started.elapsed() < Duration::from_secs(15));

        server.abort();
    }
}
