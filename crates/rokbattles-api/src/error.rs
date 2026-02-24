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
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// Build a bad request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    /// Build a generic unauthorized error response.
    pub fn unauthorized() -> Self {
        Self::Unauthorized
    }

    /// Build an internal server error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn body(&self) -> ErrorResponse {
        match self {
            ApiError::BadRequest(message) => ErrorResponse {
                error: message.clone(),
            },
            ApiError::Unauthorized => ErrorResponse {
                error: "unauthorized".to_string(),
            },
            ApiError::Internal(_) => ErrorResponse {
                error: "internal-server-error".to_string(),
            },
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
        let body = Json(self.body());
        (status, body).into_response()
    }
}
