use reqwest::StatusCode;

/// Things that can go wrong while talking to ROK Game Tools.
#[derive(Debug, thiserror::Error)]
pub enum RokGtError {
    /// Reqwest failed before the API returned a usable response.
    #[error("http request failed")]
    Http(#[from] reqwest::Error),
    /// The response body was not valid JSON.
    #[error("failed to decode json response")]
    Json(#[from] serde_json::Error),
    /// The system clock is earlier than the Unix epoch.
    #[error("system clock is before the Unix epoch")]
    InvalidSystemTime,
    /// The API returned a non-success HTTP status.
    #[error("api returned http status {status}: {body}")]
    HttpStatus {
        /// HTTP status code.
        status: StatusCode,
        /// Truncated response body.
        body: String,
    },
    /// The platform rejected the current Passport token.
    #[error("api authentication failed: {message}")]
    AuthRequired {
        /// Authentication failure message from the API.
        message: String,
    },
    /// The API returned an application-level error.
    #[error("api returned code {code}: {message}")]
    Api {
        /// API code.
        code: i64,
        /// API message.
        message: String,
    },
    /// A required response field was missing.
    #[error("api response is missing field {0}")]
    MissingField(&'static str),
    /// The API returned a date in an unexpected format.
    #[error("api response has invalid date {0}")]
    InvalidDate(String),
    /// Date arithmetic overflowed.
    #[error("date calculation is out of range")]
    DateOutOfRange,
    /// The account has no role we can bind.
    #[error("authenticated account has no bindable roles")]
    NoRoles,
    /// `latestServerIds` did not return `data.server_ids`.
    #[error("could not parse server IDs from latestServerIds response")]
    InvalidServerIds,
    /// `kindomMember` did not return `data` as an array.
    #[error("could not parse member records for kingdom {0}")]
    InvalidMembers(u32),
}

impl RokGtError {
    /// Whether this error should trigger one fresh login and retry.
    pub fn is_auth_failure(&self) -> bool {
        matches!(self, RokGtError::AuthRequired { .. })
    }
}
