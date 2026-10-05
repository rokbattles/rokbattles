//! HTTP routing and endpoint-specific request validation.

mod headers;
mod rate_limit;
mod relay;
mod upload;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{MethodRouter, get, post},
};

use crate::state::AppState;

/// Builds the router with a 25 MiB request body limit for both upload endpoints.
pub(crate) fn router(state: Arc<AppState>) -> Router {
    endpoints(state.config.rate_limit, post(upload::upload), post(relay::upload)).with_state(state)
}

fn endpoints<S>(rate_limit: bool, upload: MethodRouter<S>, relay: MethodRouter<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let upload = if rate_limit { rate_limit::apply(upload) } else { upload };

    Router::new()
        .route("/health", get(health))
        .route("/v2/upload", upload)
        .route("/v2/relay/upload", relay)
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024))
}

/// Liveness only; does not probe MongoDB or ClamAV.
async fn health() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        extract::{FromRequest, Multipart},
        http::{Request, StatusCode, header},
        response::Response,
    };
    use tower::ServiceExt;

    use super::*;

    fn test_router(rate_limit: bool) -> Router {
        endpoints(
            rate_limit,
            post(|_multipart: Multipart| async { StatusCode::CREATED }),
            post(|| async { StatusCode::ACCEPTED }),
        )
    }

    async fn request(app: &Router, path: &str, ip: Option<&str>) -> Response {
        let mut request =
            Request::builder().method(if path == "/health" { "GET" } else { "POST" }).uri(path);

        if let Some(ip) = ip {
            request = request.header("cf-connecting-ip", ip);
        }

        app.clone()
            .oneshot(request.body(Body::empty()).expect("test request"))
            .await
            .expect("router response")
    }

    #[tokio::test]
    async fn rate_limits_before_multipart_validation_with_json_and_retry_after() {
        let app = test_router(true);

        for _ in 0..600 {
            let response = request(&app, "/v2/upload", Some("192.0.2.1")).await;

            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let response = request(&app, "/v2/upload", Some("192.0.2.1")).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).expect("retry delay"), "60");
        assert_eq!(response.headers().get(header::CONTENT_TYPE).expect("JSON"), "application/json");

        let body = to_bytes(response.into_body(), 1024).await.expect("response body");

        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("JSON error"),
            serde_json::json!({ "error": "upload rate limit exceeded" })
        );

        assert_eq!(
            request(&app, "/v2/upload", Some("192.0.2.2")).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn enabled_limit_requires_a_valid_cloudflare_header() {
        let app = test_router(true);

        for ip in [None, Some("invalid")] {
            let response = request(&app, "/v2/upload", ip).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);

            let body = to_bytes(response.into_body(), 1024).await.expect("response body");
            let error: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");

            assert_eq!(
                error.get("error"),
                Some(&serde_json::json!(if ip.is_none() {
                    "missing cf-connecting-ip"
                } else {
                    "invalid cf-connecting-ip"
                }))
            );
        }
    }

    #[tokio::test]
    async fn relay_and_health_bypass_the_upload_quota_and_header_requirement() {
        let app = test_router(true);

        for _ in 0..601 {
            let _response = request(&app, "/v2/upload", Some("192.0.2.1")).await;
        }

        for ip in [None, Some("invalid"), Some("192.0.2.1")] {
            assert_eq!(request(&app, "/v2/relay/upload", ip).await.status(), StatusCode::ACCEPTED);
            assert_eq!(request(&app, "/health", ip).await.status(), StatusCode::OK);
        }

        for _ in 0..601 {
            assert_eq!(
                request(&app, "/v2/relay/upload", Some("192.0.2.2")).await.status(),
                StatusCode::ACCEPTED
            );
        }

        assert_eq!(
            request(&app, "/v2/upload", Some("192.0.2.2")).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn disabled_limit_allows_uploads_without_a_valid_ip_or_quota() {
        let app = test_router(false);

        for ip in [None, Some("invalid"), Some("192.0.2.1")] {
            for _ in 0..601 {
                let mut request = Request::builder()
                    .method("POST")
                    .uri("/v2/upload")
                    .header(header::CONTENT_TYPE, "multipart/form-data; boundary=mail-boundary");

                if let Some(ip) = ip {
                    request = request.header("cf-connecting-ip", ip);
                }

                let response = app
                    .clone()
                    .oneshot(request.body(Body::from("--mail-boundary--\r\n")).expect("request"))
                    .await
                    .expect("response");

                assert_eq!(response.status(), StatusCode::CREATED);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_requests_share_one_upload_quota() {
        let app = test_router(true);
        let mut requests = tokio::task::JoinSet::new();

        for _ in 0..700 {
            let app = app.clone();

            requests.spawn(async move {
                request(&app, "/v2/upload", Some("2001:db8::1")).await.status()
            });
        }

        let mut admitted = 0;
        let mut limited = 0;

        while let Some(result) = requests.join_next().await {
            match result.expect("request task") {
                StatusCode::BAD_REQUEST => admitted += 1,
                StatusCode::TOO_MANY_REQUESTS => limited += 1,
                status => assert_eq!(status, StatusCode::BAD_REQUEST, "unexpected status"),
            }
        }

        assert_eq!((admitted, limited), (600, 100));
    }

    pub(super) async fn multipart(body: &str) -> Multipart {
        let request = Request::builder()
            .header("content-type", "multipart/form-data; boundary=mail-boundary")
            .body(Body::from(body.to_string()))
            .expect("multipart request");

        Multipart::from_request(request, &()).await.expect("multipart extractor")
    }
}
