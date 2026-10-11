//! Response hardening shared by every ingress route and routing error.

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, header},
    middleware::Next,
    response::Response,
};

const CROSS_ORIGIN_RESOURCE_POLICY: &str = "cross-origin-resource-policy";
const STRICT_TRANSPORT_SECURITY: &str = "strict-transport-security";
const X_CONTENT_TYPE_OPTIONS: &str = "x-content-type-options";

/// Applies API-appropriate response headers after the inner service has completed.
///
/// The public ingress host is HTTPS-only at the reverse proxy. HSTS is emitted here as
/// defense-in-depth for responses that reach clients through that edge; TLS termination,
/// HTTP-to-HTTPS redirects, and direct-origin isolation remain edge responsibilities.
pub(super) async fn response_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    apply(response.headers_mut());
    response
}

fn apply(headers: &mut HeaderMap) {
    // Upload acknowledgements and errors can contain mail IDs or diagnostic details.
    headers.entry(header::CACHE_CONTROL).or_insert(HeaderValue::from_static("no-store"));
    // Ingress has native clients only and its responses are not embeddable browser resources.
    headers.entry(CROSS_ORIGIN_RESOURCE_POLICY).or_insert(HeaderValue::from_static("same-origin"));
    // HSTS is host-scoped, so individual routes must not weaken or clear the policy.
    headers.insert(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000"));
    // Keep JSON errors from being reinterpreted as active content by browsers.
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        extract::{DefaultBodyLimit, Multipart, multipart::MultipartError},
        http::{Method, Request, StatusCode, header},
        middleware,
        routing::{get, post},
    };
    use tower::ServiceExt;
    use tower_http::limit::RequestBodyLimitLayer;

    use super::*;
    use crate::error::ApiError;

    async fn consume_multipart(mut multipart: Multipart) -> Result<StatusCode, MultipartError> {
        while let Some(field) = multipart.next_field().await? {
            let _bytes = field.bytes().await?;
        }
        Ok(StatusCode::CREATED)
    }

    fn app() -> Router {
        Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .route(
                "/unauthorized",
                get(|| async { Err::<StatusCode, ApiError>(ApiError::Unauthorized) }),
            )
            .route("/upload", post(consume_multipart))
            .layer(DefaultBodyLimit::disable())
            .layer(RequestBodyLimitLayer::new(64))
            .layer(middleware::from_fn(response_headers))
    }

    async fn response(
        method: Method,
        path: &str,
        content_type: Option<&str>,
        body: Body,
    ) -> Response {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(content_type) = content_type {
            request = request.header(header::CONTENT_TYPE, content_type);
        }

        app().oneshot(request.body(body).expect("request")).await.expect("router response")
    }

    fn assert_security_headers(response: &Response) {
        let headers = response.headers();
        for (name, value) in [
            (header::CACHE_CONTROL.as_str(), "no-store"),
            (CROSS_ORIGIN_RESOURCE_POLICY, "same-origin"),
            (STRICT_TRANSPORT_SECURITY, "max-age=31536000"),
            (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ] {
            assert_eq!(headers.get(name).expect("security header"), value);
            assert_eq!(headers.get_all(name).iter().count(), 1, "duplicate {name}");
        }

        // Native clients do not need CORS. Its absence keeps browser access closed by default.
        assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
        assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_CREDENTIALS));
        assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_HEADERS));
        assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_METHODS));

        let cors_vary_tokens =
            ["origin", "access-control-request-method", "access-control-request-headers"];
        assert!(headers.get_all(header::VARY).iter().all(|value| {
            value.to_str().is_ok_and(|value| {
                value.split(',').map(str::trim).all(|token| {
                    !cors_vary_tokens.iter().any(|cors| token.eq_ignore_ascii_case(cors))
                })
            })
        }));
    }

    #[test]
    fn preserves_route_policies_and_enforces_host_security_headers() {
        let mut headers = HeaderMap::new();
        for name in [
            header::CACHE_CONTROL.as_str(),
            CROSS_ORIGIN_RESOURCE_POLICY,
            STRICT_TRANSPORT_SECURITY,
            X_CONTENT_TYPE_OPTIONS,
        ] {
            headers.insert(name, HeaderValue::from_static("route-policy"));
        }

        apply(&mut headers);

        for name in [header::CACHE_CONTROL.as_str(), CROSS_ORIGIN_RESOURCE_POLICY] {
            assert_eq!(headers.get(name).expect("route policy"), "route-policy");
            assert_eq!(headers.get_all(name).iter().count(), 1, "duplicate {name}");
        }
        assert_eq!(
            headers.get(STRICT_TRANSPORT_SECURITY).expect("HSTS policy"),
            "max-age=31536000"
        );
        assert_eq!(headers.get(X_CONTENT_TYPE_OPTIONS).expect("content-type policy"), "nosniff");
        assert_eq!(headers.get_all(STRICT_TRANSPORT_SECURITY).iter().count(), 1);
        assert_eq!(headers.get_all(X_CONTENT_TYPE_OPTIONS).iter().count(), 1);
    }

    #[tokio::test]
    async fn hardens_success_and_unauthorized_responses() {
        let health = response(Method::GET, "/health", None, Body::empty()).await;
        assert_eq!(health.status(), StatusCode::OK);
        assert_security_headers(&health);

        let unauthorized = response(Method::GET, "/unauthorized", None, Body::empty()).await;
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            unauthorized.headers().get(header::WWW_AUTHENTICATE).expect("bearer challenge"),
            "Bearer realm=\"rokbattles-ingress-relay\""
        );
        assert_security_headers(&unauthorized);
    }

    #[tokio::test]
    async fn hardens_multipart_extractor_rejections() {
        let bad_request = response(Method::POST, "/upload", None, Body::empty()).await;
        assert_eq!(bad_request.status(), StatusCode::BAD_REQUEST);
        assert_security_headers(&bad_request);

        let too_large = response(
            Method::POST,
            "/upload",
            Some("multipart/form-data; boundary=mail-boundary"),
            Body::from(concat!(
                "--mail-boundary\r\n",
                "Content-Disposition: form-data; name=\"file\"; filename=\"mail\"\r\n\r\n",
                "body larger than the configured test limit\r\n",
                "--mail-boundary--\r\n",
            )),
        )
        .await;
        assert_eq!(too_large.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_security_headers(&too_large);

        let declared_too_large = app()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/upload")
                    .header(header::CONTENT_LENGTH, "65")
                    .body(Body::empty())
                    .expect("oversized request"),
            )
            .await
            .expect("oversized response");
        assert_eq!(declared_too_large.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_security_headers(&declared_too_large);
    }

    #[tokio::test]
    async fn hardens_routing_errors_without_enabling_cors() {
        let not_found = response(Method::GET, "/missing", None, Body::empty()).await;
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
        assert_security_headers(&not_found);

        let method_not_allowed = response(Method::GET, "/upload", None, Body::empty()).await;
        assert_eq!(method_not_allowed.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_security_headers(&method_not_allowed);

        let preflight = app()
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/upload")
                    .header(header::ORIGIN, "https://rokbattles.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
                    .body(Body::empty())
                    .expect("preflight request"),
            )
            .await
            .expect("preflight response");
        assert_eq!(preflight.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_security_headers(&preflight);
    }
}
