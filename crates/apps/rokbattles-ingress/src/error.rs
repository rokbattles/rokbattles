//! API error mapping for HTTP responses.

use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Errors returned by API handlers.
///
/// Request-level errors become JSON responses with an `error` field containing
/// the display message. Axum extractor rejections bypass this conversion. Relay
/// entry errors use the message in the batch result without changing its HTTP status.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("unsupported mail type: {0}")]
    UnsupportedType(String),
    #[error("decode failed: {0}")]
    DecodeFailed(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("clamav scan failed: {0}")]
    Clamav(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("relay authentication failed")]
    Unauthorized,

    #[error("upload rate limit exceeded")]
    RateLimited { retry_after_secs: u64 },

    #[error("upload rate limiter at capacity")]
    RateLimitCapacity { retry_after_secs: u64 },
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn unsupported_type(value: impl Into<String>) -> Self {
        Self::UnsupportedType(value.into())
    }

    pub fn decode_failed(message: impl Into<String>) -> Self {
        Self::DecodeFailed(message.into())
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }

    pub fn clamav(message: impl Into<String>) -> Self {
        Self::Clamav(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::UnsupportedType(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::DecodeFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::Clamav(_) => StatusCode::BAD_GATEWAY,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::RateLimitCapacity { .. } => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Database(_) | ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(ErrorResponse { error: self.to_string() });
        let mut response = (status, body).into_response();

        if let Self::RateLimited { retry_after_secs }
        | Self::RateLimitCapacity { retry_after_secs } = self
        {
            response.headers_mut().insert(header::RETRY_AFTER, retry_after_secs.into());
        }

        response
    }
}
