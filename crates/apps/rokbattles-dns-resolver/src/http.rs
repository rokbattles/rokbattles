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
use tracing::warn;

use crate::{
    DNS_MEDIA_TYPE, DoHForwarder, MAX_DNS_MESSAGE_BYTES, ResolveError, Resolver,
    resolver::IntraResolution,
};

#[derive(Debug, Deserialize)]
struct DnsQuery {
    dns: String,
}

#[derive(Clone)]
struct AppState {
    resolver: Resolver,
    forwarder: DoHForwarder,
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

/// Build the HTTP router for the iOS `/query` and Intra `/intra` endpoints.
pub fn router(resolver: Resolver, forwarder: DoHForwarder) -> Router {
    Router::new()
        .route("/query", get(get_dns_query).post(post_dns_query))
        .route("/intra", get(get_intra_query).post(post_intra_query))
        .with_state(AppState { resolver, forwarder })
        .layer(axum::extract::DefaultBodyLimit::max(MAX_DNS_MESSAGE_BYTES))
}

async fn get_dns_query(
    State(state): State<AppState>,
    Query(query): Query<DnsQuery>,
) -> Result<Response, HttpError> {
    let wire_request = decode_dns_parameter(&query.dns)?;
    resolve(&state.resolver, &wire_request)
}

async fn post_dns_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, HttpError> {
    require_dns_media_type(&headers)?;

    resolve(&state.resolver, &body)
}

async fn get_intra_query(
    State(state): State<AppState>,
    Query(query): Query<DnsQuery>,
) -> Result<Response, HttpError> {
    let wire_request = decode_dns_parameter(&query.dns)?;
    resolve_for_intra(&state, &wire_request).await
}

async fn post_intra_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, HttpError> {
    require_dns_media_type(&headers)?;

    resolve_for_intra(&state, &body).await
}

fn resolve(resolver: &Resolver, wire_request: &[u8]) -> Result<Response, HttpError> {
    let wire_response = resolver.resolve(wire_request).map_err(map_resolve_error)?;
    Ok(dns_response(wire_response))
}

async fn resolve_for_intra(state: &AppState, wire_request: &[u8]) -> Result<Response, HttpError> {
    let wire_response =
        match state.resolver.resolve_for_intra(wire_request).map_err(map_resolve_error)? {
            IntraResolution::Local(response) => response,
            IntraResolution::Forward => match state.forwarder.forward(wire_request).await {
                Ok(response) => response,
                Err(error) => {
                    warn!(%error, "upstream DoH query failed");
                    state.resolver.servfail(wire_request).map_err(map_resolve_error)?
                }
            },
        };
    Ok(dns_response(wire_response))
}

fn dns_response(wire_response: Vec<u8>) -> Response {
    let mut response = wire_response.into_response();
    response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(DNS_MEDIA_TYPE));
    response
}

fn require_dns_media_type(headers: &HeaderMap) -> Result<(), HttpError> {
    let content_type = headers.get(CONTENT_TYPE).and_then(|value| value.to_str().ok());
    if !content_type.is_some_and(|value| value.eq_ignore_ascii_case(DNS_MEDIA_TYPE)) {
        return Err(HttpError::UnsupportedMediaType);
    }
    Ok(())
}

fn map_resolve_error(error: ResolveError) -> HttpError {
    match error {
        ResolveError::Decode(_) => HttpError::InvalidDnsMessage,
        ResolveError::Encode(_) => HttpError::ResponseEncoding,
    }
}

fn decode_dns_parameter(encoded: &str) -> Result<Vec<u8>, HttpError> {
    let wire_request =
        URL_SAFE_NO_PAD.decode(encoded).map_err(|_| HttpError::InvalidDnsParameter)?;
    if wire_request.len() > MAX_DNS_MESSAGE_BYTES {
        return Err(HttpError::MessageTooLarge);
    }
    Ok(wire_request)
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        sync::mpsc::{self, Receiver, Sender, TryRecvError},
        time::Duration,
    };

    use axum::{
        body::{Body, to_bytes},
        extract::State,
        http::{Method, Request, header::ACCEPT},
        routing::post,
    };
    use hickory_proto::{
        op::{Message, MessageType, OpCode, Query, ResponseCode},
        rr::{
            Name, RData, Record, RecordType,
            rdata::{A, AAAA},
        },
    };
    use reqwest::Url;
    use tokio::task::JoinHandle;
    use tower::ServiceExt;

    use super::*;

    const TARGET_HOSTNAME: &str = "example.com";
    const RELAY_IPV4: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 10);
    const RELAY_IPV6: Ipv6Addr = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x10);
    const UPSTREAM_IPV4: Ipv4Addr = Ipv4Addr::new(198, 51, 100, 42);

    #[derive(Clone)]
    struct MockState {
        requests: Sender<ForwardedRequest>,
        delay: Duration,
    }

    #[derive(Debug)]
    struct ForwardedRequest {
        body: Vec<u8>,
        content_type: Option<HeaderValue>,
        accept: Option<HeaderValue>,
    }

    struct MockUpstream {
        url: Url,
        requests: Receiver<ForwardedRequest>,
        task: JoinHandle<()>,
    }

    impl Drop for MockUpstream {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    impl MockUpstream {
        async fn start(delay: Duration) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("mock upstream should bind");
            let address = listener.local_addr().expect("mock address should be available");
            let url = Url::parse(&format!("http://{address}/dns-query"))
                .expect("mock URL should be valid");
            let (request_tx, requests) = mpsc::channel();
            let app = Router::new()
                .route("/dns-query", post(mock_upstream_query))
                .with_state(MockState { requests: request_tx, delay });
            let task = tokio::spawn(async move {
                axum::serve(listener, app).await.expect("mock upstream should serve");
            });
            Self { url, requests, task }
        }
    }

    fn app() -> Router {
        app_with_upstream(
            Url::parse("http://127.0.0.1:9/dns-query").expect("fixture URL should be valid"),
            Duration::from_secs(1),
        )
    }

    fn app_with_upstream(upstream_url: Url, timeout: Duration) -> Router {
        let resolver = Resolver::new(TARGET_HOSTNAME, RELAY_IPV4, None);
        let forwarder = DoHForwarder::with_test_timeout(upstream_url, timeout)
            .expect("forwarder fixture should build");
        router(resolver, forwarder)
    }

    fn dns_query(name: &str) -> Vec<u8> {
        dns_query_for(name, RecordType::A)
    }

    fn dns_query_for(name: &str, record_type: RecordType) -> Vec<u8> {
        let mut message = Message::new(0x1234, MessageType::Query, OpCode::Query);
        message.metadata.recursion_desired = true;
        message.add_query(Query::query(
            Name::from_ascii(name).expect("query name fixture should be valid"),
            record_type,
        ));
        message.to_vec().expect("query fixture should encode")
    }

    async fn send_post(
        app: Router,
        path: &'static str,
        body: Vec<u8>,
        content_type: &'static str,
    ) -> Response {
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(path)
                .header(CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .expect("request fixture should build"),
        )
        .await
        .expect("router should respond")
    }

    async fn send_get(app: Router, path: &str, wire_query: &[u8]) -> Response {
        let encoded = URL_SAFE_NO_PAD.encode(wire_query);
        app.oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("{path}?dns={encoded}"))
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

    async fn mock_upstream_query(
        State(state): State<MockState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Response {
        state
            .requests
            .send(ForwardedRequest {
                body: body.to_vec(),
                content_type: headers.get(CONTENT_TYPE).cloned(),
                accept: headers.get(ACCEPT).cloned(),
            })
            .expect("test should retain mock request receiver");
        tokio::time::sleep(state.delay).await;

        let request = Message::from_vec(&body).expect("forwarded request should be valid DNS");
        let mut response = Message::response(request.metadata.id, request.metadata.op_code);
        response.add_queries(request.queries.iter().cloned());
        response.metadata.recursion_desired = request.metadata.recursion_desired;
        response.metadata.recursion_available = true;
        let query = request.queries.first().expect("forwarded request should have one question");
        response.add_answer(Record::from_rdata(
            query.name().clone(),
            60,
            RData::A(A(UPSTREAM_IPV4)),
        ));
        dns_response(response.to_vec().expect("mock response should encode"))
    }

    #[tokio::test]
    async fn ios_post_query_should_return_dns_wire_format_response() {
        let response = send_post(app(), "/query", dns_query(TARGET_HOSTNAME), DNS_MEDIA_TYPE).await;
        let status = response.status();
        let content_type = response.headers().get(CONTENT_TYPE).cloned();
        let message = decode_dns_response(response).await;

        assert_eq!(
            (status, content_type.as_ref(), message.answers.len()),
            (StatusCode::OK, Some(&HeaderValue::from_static(DNS_MEDIA_TYPE)), 1)
        );
    }

    #[tokio::test]
    async fn ios_get_query_should_return_dns_wire_format_response() {
        let response = send_get(app(), "/query", &dns_query(TARGET_HOSTNAME)).await;
        let message = decode_dns_response(response).await;

        assert_eq!(message.answers.len(), 1);
    }

    #[tokio::test]
    async fn intra_post_target_query_should_return_local_answer() {
        let response = send_post(app(), "/intra", dns_query(TARGET_HOSTNAME), DNS_MEDIA_TYPE).await;
        let message = decode_dns_response(response).await;

        assert_eq!(
            message.answers.first().map(|answer| &answer.data),
            Some(&RData::A(A(RELAY_IPV4)))
        );
    }

    #[tokio::test]
    async fn intra_get_target_query_should_return_local_answer() {
        let response = send_get(app(), "/intra", &dns_query(TARGET_HOSTNAME)).await;
        let message = decode_dns_response(response).await;

        assert_eq!(
            message.answers.first().map(|answer| &answer.data),
            Some(&RData::A(A(RELAY_IPV4)))
        );
    }

    #[tokio::test]
    async fn intra_target_aaaa_query_should_return_configured_local_answer() {
        let resolver = Resolver::new(TARGET_HOSTNAME, RELAY_IPV4, Some(RELAY_IPV6));
        let forwarder = DoHForwarder::new(
            Url::parse("http://127.0.0.1:9/dns-query").expect("fixture URL should be valid"),
        )
        .expect("forwarder fixture should build");
        let response = send_post(
            router(resolver, forwarder),
            "/intra",
            dns_query_for(TARGET_HOSTNAME, RecordType::AAAA),
            DNS_MEDIA_TYPE,
        )
        .await;
        let message = decode_dns_response(response).await;

        assert_eq!(
            message.answers.first().map(|answer| &answer.data),
            Some(&RData::AAAA(AAAA(RELAY_IPV6)))
        );
    }

    #[tokio::test]
    async fn intra_non_target_query_should_be_forwarded_to_upstream_doh() {
        let upstream = MockUpstream::start(Duration::ZERO).await;
        let query = dns_query("www.example.net");
        let response = send_post(
            app_with_upstream(upstream.url.clone(), Duration::from_secs(1)),
            "/intra",
            query.clone(),
            DNS_MEDIA_TYPE,
        )
        .await;
        let message = decode_dns_response(response).await;
        let forwarded = upstream.requests.recv().expect("upstream should receive one request");

        assert_eq!(
            (
                message.answers.first().map(|answer| &answer.data),
                forwarded.body,
                forwarded.content_type,
                forwarded.accept,
            ),
            (
                Some(&RData::A(A(UPSTREAM_IPV4))),
                query,
                Some(HeaderValue::from_static(DNS_MEDIA_TYPE)),
                Some(HeaderValue::from_static(DNS_MEDIA_TYPE)),
            )
        );
    }

    #[tokio::test]
    async fn intra_upstream_timeout_should_return_dns_servfail() {
        let upstream = MockUpstream::start(Duration::from_millis(200)).await;
        let response = send_post(
            app_with_upstream(upstream.url.clone(), Duration::from_millis(20)),
            "/intra",
            dns_query("www.example.net"),
            DNS_MEDIA_TYPE,
        )
        .await;
        let status = response.status();
        let content_type = response.headers().get(CONTENT_TYPE).cloned();
        let message = decode_dns_response(response).await;

        assert_eq!(
            (status, content_type, message.metadata.response_code, message.metadata.id),
            (
                StatusCode::OK,
                Some(HeaderValue::from_static(DNS_MEDIA_TYPE)),
                ResponseCode::ServFail,
                0x1234,
            )
        );
    }

    #[tokio::test]
    async fn ios_non_target_query_should_remain_refused_without_forwarding() {
        let upstream = MockUpstream::start(Duration::ZERO).await;
        let response = send_post(
            app_with_upstream(upstream.url.clone(), Duration::from_secs(1)),
            "/query",
            dns_query("www.example.net"),
            DNS_MEDIA_TYPE,
        )
        .await;
        let message = decode_dns_response(response).await;

        assert_eq!(
            (
                message.metadata.response_code,
                matches!(upstream.requests.try_recv(), Err(TryRecvError::Empty)),
            ),
            (ResponseCode::Refused, true)
        );
    }

    #[tokio::test]
    async fn malformed_dns_post_body_should_be_rejected() {
        let response = send_post(app(), "/query", vec![0, 1, 2], DNS_MEDIA_TYPE).await;

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
        let response =
            send_post(app(), "/query", dns_query(TARGET_HOSTNAME), "application/octet-stream")
                .await;

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
        let response =
            send_post(app(), "/query", vec![0; MAX_DNS_MESSAGE_BYTES + 1], DNS_MEDIA_TYPE).await;

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
