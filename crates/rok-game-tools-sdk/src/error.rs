use reqwest::StatusCode;
use thiserror::Error;

/// Errors returned by the ROK Game Tools SDK client.
#[derive(Debug, Error)]
pub enum RokGtError {
    #[error("invalid configuration for {field}: {reason}")]
    InvalidConfig { field: &'static str, reason: &'static str },
    #[error("invalid request value for {field}: {reason}")]
    InvalidRequest { field: &'static str, reason: &'static str },
    #[error("invalid header value for {header}")]
    InvalidHeaderValue {
        header: &'static str,
        #[source]
        source: reqwest::header::InvalidHeaderValue,
    },
    #[error("failed to build HTTP client: {0}")]
    ClientBuild(#[source] reqwest::Error),
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("HTTP status {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
    #[error("failed to decode response JSON: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("API returned error code {code}: {message}")]
    Api { code: u32, message: String },
}
