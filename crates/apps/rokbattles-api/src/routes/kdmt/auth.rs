use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{error::ApiError, state::AppState};

pub(super) struct KdmtAuthorization;

impl FromRequestParts<Arc<AppState>> for KdmtAuthorization {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        authorize(&parts.headers, state.kdmt_token.as_deref())?;

        Ok(Self)
    }
}

fn authorize(headers: &HeaderMap, expected: Option<&str>) -> Result<(), ApiError> {
    let expected =
        expected.filter(|token| !token.trim().is_empty()).ok_or_else(ApiError::unauthorized)?;

    let mut values = headers.get_all(AUTHORIZATION).iter();
    let (scheme, token) = values
        .next()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .ok_or_else(ApiError::unauthorized)?;

    if values.next().is_some() || !scheme.eq_ignore_ascii_case("Bearer") || token.is_empty() {
        return Err(ApiError::unauthorized());
    }

    // Fixed-size digests avoid leaking credential contents or length in the comparison.
    let expected = Sha256::digest(expected.as_bytes());
    let actual = Sha256::digest(token.as_bytes());

    if !bool::from(expected.ct_eq(&actual)) {
        return Err(ApiError::unauthorized());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn rejects_missing_wrong_blank_and_duplicate_credentials() {
        for value in [
            None,
            Some("Bearer pk-secret"),
            Some("Basic kdmt-secret"),
            Some("Bearer "),
            Some("Bearer  kdmt-secret"),
        ] {
            let mut headers = HeaderMap::new();

            if let Some(value) = value {
                headers.insert(AUTHORIZATION, HeaderValue::from_static(value));
            }

            assert!(authorize(&headers, Some("kdmt-secret")).is_err(), "{value:?}");
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("bEaReR kdmt-secret"));
        authorize(&headers, Some("kdmt-secret")).unwrap();

        for expected in [None, Some(""), Some("  ")] {
            assert!(authorize(&headers, expected).is_err());
        }

        headers.append(AUTHORIZATION, HeaderValue::from_static("Bearer kdmt-secret"));
        assert!(authorize(&headers, Some("kdmt-secret")).is_err());
    }
}
