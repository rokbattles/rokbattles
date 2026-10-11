//! API error mapping for HTTP responses.

use axum::{
    Json,
    extract::multipart::MultipartError,
    http::{HeaderValue, StatusCode, header},
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
    #[error("{0}")]
    PayloadTooLarge(String),
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
            ApiError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::UnsupportedType(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::DecodeFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::Clamav(_) => StatusCode::BAD_GATEWAY,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Database(_) | ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<MultipartError> for ApiError {
    fn from(error: MultipartError) -> Self {
        let status = error.status();
        match status {
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge(error.body_text()),
            StatusCode::BAD_REQUEST => Self::BadRequest(error.body_text()),
            _ => Self::Internal("multipart stream failed".to_string()),
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
        let requires_bearer_token = matches!(self, Self::Unauthorized);
        let body = Json(ErrorResponse { error: self.to_string() });
        let mut response = (status, body).into_response();

        // RFC 9110 requires a challenge on 401 responses. Keep the challenge generic so a
        // rejected relay token never leaks validation details.
        if requires_bearer_token {
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer realm=\"rokbattles-ingress-relay\""),
            );
        }

        response
    }
}
