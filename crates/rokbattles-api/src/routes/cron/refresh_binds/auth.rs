use axum::http::HeaderMap;

const CRON_SECRET_HEADER: &str = "x-cron-secret";

/// Returns true when the request includes the expected cron secret.
pub(super) fn is_authorized_request(headers: &HeaderMap, expected_secret: &str) -> bool {
    headers
        .get(CRON_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|provided| provided == expected_secret)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn authorizes_matching_secret() {
        let mut headers = HeaderMap::new();
        headers.insert(CRON_SECRET_HEADER, HeaderValue::from_static("secret"));

        assert!(is_authorized_request(&headers, "secret"));
    }

    #[test]
    fn rejects_missing_or_invalid_secret() {
        let mut headers = HeaderMap::new();
        headers.insert(CRON_SECRET_HEADER, HeaderValue::from_static("wrong"));

        assert!(!is_authorized_request(&HeaderMap::new(), "secret"));
        assert!(!is_authorized_request(&headers, "secret"));
    }
}
