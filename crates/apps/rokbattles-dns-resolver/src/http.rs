//! RFC 8484 HTTP request and response handling.

use axum::{
    Router,
    body::Bytes,
    extract::{Query, State},
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_TYPE, HeaderValue},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

use crate::Resolver;

const DNS_MEDIA_TYPE: &str = "application/dns-message";
const MAX_DNS_MESSAGE_BYTES: usize = 65_535;

#[derive(Debug, Deserialize)]
struct DnsQuery {
    dns: String,
}

#[derive(Debug, thiserror::Error)]
enum HttpError {
    #[error("the dns query parameter must be unpadded base64url")]
    InvalidDnsParameter,
    #[error("Content-Type must be application/dns-message")]
    UnsupportedMediaType,
    #[error("DNS message exceeds 65535 bytes")]
    MessageTooLarge,
    #[error("request body is not a valid DNS message")]
    InvalidDnsMessage,
    #[error("failed to encode DNS response")]
    ResponseEncoding,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::InvalidDnsParameter | Self::InvalidDnsMessage => StatusCode::BAD_REQUEST,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::MessageTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::ResponseEncoding => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

/// Build the HTTP router for the RFC 8484 `/query` endpoint.
pub fn router(resolver: Resolver) -> Router {
    Router::new()
        .route("/query", get(get_dns_query).post(post_dns_query))
        .with_state(resolver)
        .layer(axum::extract::DefaultBodyLimit::max(MAX_DNS_MESSAGE_BYTES))
}

async fn get_dns_query(
    State(resolver): State<Resolver>,
    Query(query): Query<DnsQuery>,
) -> Result<Response, HttpError> {
    let wire_request = decode_dns_parameter(&query.dns)?;
    resolve(&resolver, &wire_request)
}

async fn post_dns_query(
    State(resolver): State<Resolver>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, HttpError> {
    let content_type = headers.get(CONTENT_TYPE).and_then(|value| value.to_str().ok());
    if !content_type.is_some_and(|value| value.eq_ignore_ascii_case(DNS_MEDIA_TYPE)) {
        return Err(HttpError::UnsupportedMediaType);
    }

    resolve(&resolver, &body)
}

fn resolve(resolver: &Resolver, wire_request: &[u8]) -> Result<Response, HttpError> {
    let wire_response = resolver.resolve(wire_request).map_err(|error| match error {
        crate::ResolveError::Decode(_) => HttpError::InvalidDnsMessage,
        crate::ResolveError::Encode(_) => HttpError::ResponseEncoding,
    })?;
    let mut response = wire_response.into_response();
    response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(DNS_MEDIA_TYPE));
    Ok(response)
}

fn decode_dns_parameter(encoded: &str) -> Result<Vec<u8>, HttpError> {
    let wire_request =
        URL_SAFE_NO_PAD.decode(encoded).map_err(|_error| HttpError::InvalidDnsParameter)?;
    if wire_request.len() > MAX_DNS_MESSAGE_BYTES {
        return Err(HttpError::MessageTooLarge);
    }
    Ok(wire_request)
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request},
    };
    use hickory_proto::{
        op::{Message, MessageType, OpCode, Query},
        rr::{Name, RecordType},
    };
    use tower::ServiceExt;

    use super::*;

    const TARGET_HOSTNAME: &str = "example.com";
    const RELAY_IPV4: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 10);

    fn app() -> Router {
        router(Resolver::new(TARGET_HOSTNAME, RELAY_IPV4, None))
    }

    fn dns_query() -> Vec<u8> {
        let mut message = Message::new(0x1234, MessageType::Query, OpCode::Query);
        message.metadata.recursion_desired = true;
        message.add_query(Query::query(
            Name::from_ascii(TARGET_HOSTNAME).expect("query name fixture should be valid"),
            RecordType::A,
        ));
        message.to_vec().expect("query fixture should encode")
    }

    async fn send_post(body: Vec<u8>, content_type: &'static str) -> Response {
        app()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/query")
                    .header(CONTENT_TYPE, content_type)
                    .body(Body::from(body))
                    .expect("request fixture should build"),
            )
            .await
            .expect("router should respond")
    }

    async fn send_get(wire_query: &[u8]) -> Response {
        let encoded = URL_SAFE_NO_PAD.encode(wire_query);
        app()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/query?dns={encoded}"))
                    .body(Body::empty())
                    .expect("request fixture should build"),
            )
            .await
            .expect("router should respond")
    }

    async fn decode_dns_response(response: Response) -> Message {
        let body = to_bytes(response.into_body(), MAX_DNS_MESSAGE_BYTES)
            .await
            .expect("response body should be readable");
        Message::from_vec(&body).expect("response should contain a DNS message")
    }

    #[tokio::test]
    async fn post_query_should_return_dns_wire_format_response() {
        let response = send_post(dns_query(), DNS_MEDIA_TYPE).await;
        let status = response.status();
        let content_type = response.headers().get(CONTENT_TYPE).cloned();
        let message = decode_dns_response(response).await;

        assert_eq!(
            (status, content_type.as_ref(), message.answers.len()),
            (StatusCode::OK, Some(&HeaderValue::from_static(DNS_MEDIA_TYPE)), 1)
        );
    }

    #[tokio::test]
    async fn get_query_should_return_dns_wire_format_response() {
        let response = send_get(&dns_query()).await;
        let message = decode_dns_response(response).await;

        assert_eq!(message.answers.len(), 1);
    }

    #[tokio::test]
    async fn malformed_dns_post_body_should_be_rejected() {
        let response = send_post(vec![0, 1, 2], DNS_MEDIA_TYPE).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_get_parameter_should_be_rejected() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/query?dns=not+base64")
                    .body(Body::empty())
                    .expect("request fixture should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_without_dns_media_type_should_be_rejected() {
        let response = send_post(dns_query(), "application/octet-stream").await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[test]
    fn oversized_decoded_get_message_should_be_rejected() {
        let encoded = URL_SAFE_NO_PAD.encode(vec![0; MAX_DNS_MESSAGE_BYTES + 1]);
        let error =
            decode_dns_parameter(&encoded).expect_err("decoded message should exceed DNS limit");

        assert!(matches!(error, HttpError::MessageTooLarge));
    }

    #[tokio::test]
    async fn oversized_post_message_should_be_rejected() {
        let response = send_post(vec![0; MAX_DNS_MESSAGE_BYTES + 1], DNS_MEDIA_TYPE).await;

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn unsupported_http_method_should_return_method_not_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/query")
                    .body(Body::empty())
                    .expect("request fixture should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn old_dns_query_path_should_not_be_served() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/dns-query")
                    .body(Body::empty())
                    .expect("request fixture should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
