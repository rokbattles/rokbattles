//! Per-process rolling upload quotas, keyed by a trusted proxy's CF-Connecting-IP.

use std::{
    collections::{BTreeSet, HashMap, VecDeque, hash_map::Entry},
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::MethodRouter,
};

use crate::error::ApiError;

const MAX_REQUESTS: usize = 600;
const WINDOW: Duration = Duration::from_secs(60);
const MAX_TRACKED_IPS: usize = 4_096;
const CLEANUP_BATCH: usize = 32;

pub(super) fn apply<S>(route: MethodRouter<S>) -> MethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    let limiter = Arc::new(Mutex::new(RateLimiter::new()));

    route.route_layer(from_fn_with_state(limiter, enforce))
}

async fn enforce(
    State(limiter): State<Arc<Mutex<RateLimiter>>>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let ip = client_ip(request.headers())?;

    {
        let mut limiter = limiter
            .lock()
            .map_err(|_error| ApiError::internal("upload rate limiter unavailable"))?;

        limiter.check(ip, Instant::now()).map_err(ApiError::from)?;
    }

    Ok(next.run(request).await)
}

fn client_ip(headers: &HeaderMap) -> Result<IpAddr, ApiError> {
    let mut values = headers.get_all("cf-connecting-ip").iter();
    let value = values.next().ok_or_else(|| ApiError::bad_request("missing cf-connecting-ip"))?;

    if values.next().is_some() {
        return Err(ApiError::bad_request("invalid cf-connecting-ip"));
    }

    value
        .to_str()
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok())
        .map(|ip| ip.to_canonical())
        .ok_or_else(|| ApiError::bad_request("invalid cf-connecting-ip"))
}

struct RateLimiter {
    #[expect(clippy::disallowed_types, reason = "Client IP keys need collision-resistant hashing")]
    clients: HashMap<IpAddr, ClientWindow>,
    expirations: BTreeSet<(Instant, IpAddr)>,
}

struct ClientWindow {
    requests: VecDeque<Instant>,
    expires_at: Instant,
}

#[derive(Debug, PartialEq, Eq)]
enum Rejection {
    RateLimited(Duration),
    AtCapacity,
}

impl From<Rejection> for ApiError {
    fn from(rejection: Rejection) -> Self {
        match rejection {
            Rejection::RateLimited(retry_after) => Self::RateLimited {
                retry_after_secs: retry_after.as_secs()
                    + u64::from(retry_after.subsec_nanos() != 0),
            },
            Rejection::AtCapacity => Self::RateLimitCapacity { retry_after_secs: WINDOW.as_secs() },
        }
    }
}

impl RateLimiter {
    fn new() -> Self {
        Self { clients: Default::default(), expirations: BTreeSet::new() }
    }

    fn check(&mut self, ip: IpAddr, now: Instant) -> Result<(), Rejection> {
        self.remove_expired(now);

        let client = match self.clients.entry(ip) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                if self.expirations.len() >= MAX_TRACKED_IPS {
                    return Err(Rejection::AtCapacity);
                }

                entry.insert(ClientWindow { requests: VecDeque::new(), expires_at: now + WINDOW })
            }
        };

        while client.requests.front().is_some_and(|first| now.duration_since(*first) >= WINDOW) {
            client.requests.pop_front();
        }

        if client.requests.len() >= MAX_REQUESTS
            && let Some(first) = client.requests.front()
        {
            return Err(Rejection::RateLimited(WINDOW.saturating_sub(now.duration_since(*first))));
        }

        self.expirations.remove(&(client.expires_at, ip));

        client.requests.push_back(now);
        client.expires_at = now + WINDOW;

        self.expirations.insert((client.expires_at, ip));

        Ok(())
    }

    fn remove_expired(&mut self, now: Instant) {
        for _ in 0..CLEANUP_BATCH {
            let Some(&(expires_at, ip)) = self.expirations.first() else {
                break;
            };

            if expires_at > now {
                break;
            }

            self.expirations.pop_first();
            self.clients.remove(&ip);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{HeaderValue, StatusCode, header},
        response::IntoResponse,
        routing::post,
    };
    use tower::ServiceExt;

    use super::*;

    fn ip(value: &str) -> IpAddr {
        value.parse().expect("test IP")
    }

    fn numbered_ip(index: usize) -> IpAddr {
        Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, index.try_into().expect("test IP index")).into()
    }

    fn headers(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", value.parse().expect("test header"));

        headers
    }

    #[test]
    fn rejects_request_601_and_keeps_other_ips_independent() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();

        for _ in 0..600 {
            limiter.check(ip("192.0.2.1"), now).expect("within quota");
        }

        assert_eq!(limiter.check(ip("192.0.2.1"), now), Err(Rejection::RateLimited(WINDOW)));

        limiter.check(ip("192.0.2.2"), now).expect("separate IPv4 quota");
        limiter.check(ip("2001:db8::1"), now).expect("separate IPv6 quota");
    }

    #[test]
    fn expires_each_request_after_60_seconds_without_a_boundary_burst() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();
        let client = ip("192.0.2.1");

        limiter.check(client, now).expect("first request");
        for _ in 1..600 {
            limiter.check(client, now + Duration::from_secs(30)).expect("within quota");
        }

        assert_eq!(
            limiter.check(client, now + Duration::from_millis(59_999)),
            Err(Rejection::RateLimited(Duration::from_millis(1)))
        );

        limiter.check(client, now + WINDOW).expect("oldest request expired");
        assert_eq!(
            limiter.check(client, now + WINDOW),
            Err(Rejection::RateLimited(Duration::from_secs(30)))
        );

        for _ in 1..600 {
            limiter.check(client, now + Duration::from_secs(90)).expect("later requests expired");
        }

        assert_eq!(
            limiter.check(client, now + Duration::from_secs(90)),
            Err(Rejection::RateLimited(Duration::from_secs(30)))
        );
    }

    #[test]
    fn cleanup_removes_idle_ips_and_preserves_active_quotas() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();
        let idle = ip("192.0.2.1");
        let active = ip("192.0.2.2");

        limiter.check(idle, now).expect("idle IP request");
        for _ in 0..600 {
            limiter.check(active, now + Duration::from_secs(30)).expect("active IP quota");
        }

        assert_eq!(
            limiter.check(active, now + WINDOW),
            Err(Rejection::RateLimited(Duration::from_secs(30)))
        );
        assert!(!limiter.clients.contains_key(&idle));
        assert_eq!(limiter.clients.len(), 1);

        limiter.check(idle, now + Duration::from_secs(120)).expect("new quota");
        assert!(!limiter.clients.contains_key(&active));
    }

    #[test]
    fn capacity_rejects_new_ips_without_evicting_or_resetting_existing_quotas() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();
        let existing = numbered_ip(0);

        for index in 0..MAX_TRACKED_IPS {
            limiter.check(numbered_ip(index), now).expect("capacity available");
        }

        for _ in 1..600 {
            limiter.check(existing, now).expect("existing quota remains usable at capacity");
        }

        for index in MAX_TRACKED_IPS..MAX_TRACKED_IPS + 100 {
            assert_eq!(limiter.check(numbered_ip(index), now), Err(Rejection::AtCapacity));
        }

        assert_eq!(limiter.clients.len(), MAX_TRACKED_IPS);
        assert_eq!(limiter.expirations.len(), MAX_TRACKED_IPS);
        assert_eq!(limiter.check(existing, now), Err(Rejection::RateLimited(WINDOW)));
    }

    #[test]
    fn expiry_frees_capacity_with_bounded_cleanup_per_request() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();

        for index in 0..MAX_TRACKED_IPS {
            limiter.check(numbered_ip(index), now).expect("capacity available");
        }

        limiter.check(numbered_ip(MAX_TRACKED_IPS), now + WINDOW).expect("expired capacity reused");

        assert_eq!(limiter.clients.len(), MAX_TRACKED_IPS - CLEANUP_BATCH + 1);
        assert_eq!(limiter.expirations.len(), limiter.clients.len());
    }

    #[test]
    fn refreshing_a_client_keeps_exactly_one_expiry_and_a_bounded_history() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();
        let client = numbered_ip(0);

        for seconds in 0..1_000 {
            limiter.check(client, now + Duration::from_secs(seconds)).expect("within quota");

            assert_eq!(limiter.clients.len(), 1);
            assert_eq!(limiter.expirations.len(), 1);
        }

        assert_eq!(limiter.clients.get(&client).expect("tracked client").requests.len(), 60);
    }

    #[test]
    fn refreshes_expired_client_beyond_cleanup_batch_without_leaving_a_stale_expiry() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();

        for index in 0..CLEANUP_BATCH * 2 {
            limiter.check(numbered_ip(index), now).expect("initial request");
        }

        let client = numbered_ip(CLEANUP_BATCH * 2 - 1);
        limiter.check(client, now + WINDOW).expect("refresh expired client");

        assert!(!limiter.expirations.contains(&(now + WINDOW, client)));
        assert!(limiter.expirations.contains(&(now + WINDOW + WINDOW, client)));
        assert_eq!(limiter.expirations.len(), limiter.clients.len());

        limiter.check(client, now + WINDOW).expect("cleanup preserves refreshed client");

        assert_eq!(limiter.clients.get(&client).expect("tracked client").requests.len(), 2);
        assert_eq!(limiter.expirations.len(), 1);
    }

    #[tokio::test]
    async fn full_limiter_returns_503_with_retry_after_before_the_handler() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new();

        for index in 0..MAX_TRACKED_IPS {
            limiter.check(numbered_ip(index), now).expect("capacity available");
        }

        let app = Router::new().route(
            "/v2/upload",
            post(|| async { StatusCode::CREATED })
                .route_layer(from_fn_with_state(Arc::new(Mutex::new(limiter)), enforce)),
        );
        let request = Request::builder()
            .method("POST")
            .uri("/v2/upload")
            .header("cf-connecting-ip", numbered_ip(MAX_TRACKED_IPS).to_string())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers().get(header::RETRY_AFTER).expect("retry delay"), "60");

        let body = to_bytes(response.into_body(), 1024).await.expect("response body");

        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("JSON error"),
            serde_json::json!({ "error": "upload rate limiter at capacity" })
        );
    }

    #[test]
    fn retry_after_rounds_fractional_waits_up_to_a_whole_second() {
        let response =
            ApiError::from(Rejection::RateLimited(Duration::from_millis(1))).into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).expect("retry delay"), "1");
    }

    #[test]
    fn parses_ipv4_ipv6_and_normalizes_equivalent_addresses() {
        for value in ["192.0.2.1", "::ffff:192.0.2.1"] {
            assert_eq!(client_ip(&headers(value)).expect("IPv4"), ip("192.0.2.1"));
        }

        for value in ["2001:db8::abcd", "2001:0DB8:0:0:0:0:0:ABCD"] {
            assert_eq!(client_ip(&headers(value)).expect("IPv6"), ip("2001:db8::abcd"));
        }
    }

    #[test]
    fn rejects_missing_malformed_and_duplicate_ip_headers() {
        let mut cases = vec![HeaderMap::new()];
        for value in ["", "unknown", "192.0.2.1:1234", "192.0.2.1, 192.0.2.2", "[2001:db8::1]"] {
            cases.push(headers(value));
        }

        let mut duplicate = headers("192.0.2.1");
        duplicate.append("cf-connecting-ip", HeaderValue::from_static("192.0.2.2"));
        cases.push(duplicate);

        let mut non_ascii = HeaderMap::new();
        non_ascii.insert("cf-connecting-ip", HeaderValue::from_bytes(b"\xff").expect("header"));
        cases.push(non_ascii);

        for headers in cases {
            assert!(matches!(client_ip(&headers), Err(ApiError::BadRequest(_))));
        }
    }
}
