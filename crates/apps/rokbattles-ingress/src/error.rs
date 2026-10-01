//! API error mapping for HTTP responses.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
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
            ApiError::UnsupportedType(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::DecodeFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::Clamav(_) => StatusCode::BAD_GATEWAY,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
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
        let message = match self {
            Self::BadRequest(_) => "invalid request",
            Self::UnsupportedType(_) => "unsupported mail type",
            Self::DecodeFailed(_) => "invalid mail data",
            Self::Database(_) | Self::Internal(_) => "service unavailable",
            Self::Clamav(_) => "scan unavailable",
            Self::Unauthorized => "unauthorized",
        };
        let body = Json(ErrorResponse { error: message.to_string() });
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn server_errors_never_echo_internal_details() {
        let response = ApiError::database("mongodb://private-host/secret-token").into_response();
        let body = axum::body::to_bytes(response.into_body(), 1024).await.expect("body");
        assert_eq!(body.as_ref(), br#"{"error":"service unavailable"}"#);
    }
}
