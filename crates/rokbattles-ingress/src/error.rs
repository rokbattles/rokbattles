//! API error mapping for HTTP responses.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Errors returned by API handlers.
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
        let body = Json(ErrorResponse {
            error: self.to_string(),
        });
        (status, body).into_response()
    }
}
