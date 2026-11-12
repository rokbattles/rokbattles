//! IP-based rate limiting for HTTP handlers.
//!
//! In [`RateLimitMode::Cloudflare`], requests are identified by `CF-Connecting-IP`.
//! IPv4 addresses have individual buckets. IPv6 addresses share a bucket per `/64`.
//! The origin must restrict traffic to trusted proxies that preserve this header.
//!
//! Buckets are local to each limiter and reset when the process restarts.

use std::{
    net::{IpAddr, Ipv6Addr},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

const MAX_CLIENTS: usize = 25_000;
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// Controls whether requests are rate limited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitMode {
    /// Disables rate limiting and client IP validation.
    Local,
    /// Identifies clients by `CF-Connecting-IP`.
    ///
    /// Missing, malformed, or repeated headers are rejected.
    Cloudflare,
}

impl FromStr for RateLimitMode {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local" => Ok(Self::Local),
            "cloudflare" => Ok(Self::Cloudflare),
            _ => Err("expected local or cloudflare"),
        }
    }
}

struct Buckets {
    // The Generic Cell Rate Algorithm (GCRA) tracks when each client's bucket
    // will be full again.
    #[expect(
        clippy::disallowed_types,
        reason = "Randomized hashing protects against HashDoS from untrusted IP addresses"
    )]
    clients: std::collections::HashMap<IpAddr, Instant>,
    next_cleanup: Instant,
}

/// Tracks a request budget for each client network.
pub(crate) struct RateLimiter {
    mode: RateLimitMode,
    burst: u32,
    refill_every: Duration,
    max_clients: usize,
    buckets: Mutex<Buckets>,
}

impl RateLimiter {
    /// Creates a limiter with capacity for an initial burst of `burst` requests.
    ///
    /// Capacity refills at one request per `refill_every`, up to `burst`.
    pub(crate) fn new(mode: RateLimitMode, burst: u32, refill_every: Duration) -> Self {
        Self {
            mode,
            burst,
            refill_every,
            max_clients: MAX_CLIENTS,
            buckets: Mutex::new(Buckets {
                clients: Default::default(),
                next_cleanup: Instant::now() + CLEANUP_INTERVAL,
            }),
        }
    }

    /// Checks and consumes the client's allowance at `now`.
    fn check_at(&self, headers: &HeaderMap, now: Instant) -> Result<(), Rejection> {
        if self.mode == RateLimitMode::Local {
            return Ok(());
        }

        let ip = cloudflare_ip(headers)?;

        if self.burst == 0 || self.refill_every.is_zero() {
            return Err(Rejection::Unavailable);
        }

        let tolerance =
            self.refill_every.checked_mul(self.burst - 1).ok_or(Rejection::Unavailable)?;

        let mut buckets = self.buckets.lock().map_err(|_error| Rejection::Unavailable)?;

        if now >= buckets.next_cleanup {
            // Removing a depleted bucket would replenish its allowance early.
            buckets.clients.retain(|_, arrival| *arrival > now);
            buckets.next_cleanup =
                now.checked_add(CLEANUP_INTERVAL).ok_or(Rejection::Unavailable)?;
        }

        let arrival = match buckets.clients.get(&ip) {
            Some(arrival) => *arrival,
            None if buckets.clients.len() >= self.max_clients => {
                return Err(Rejection::Limited(
                    buckets.next_cleanup.saturating_duration_since(now),
                ));
            }
            None => now,
        };
        let debt = arrival.saturating_duration_since(now);

        if debt > tolerance {
            // Keep the stored time unchanged so retries cannot extend the wait.
            return Err(Rejection::Limited(debt.saturating_sub(tolerance)));
        }

        let next = arrival.max(now).checked_add(self.refill_every).ok_or(Rejection::Unavailable)?;
        buckets.clients.insert(ip, next);

        Ok(())
    }
}

fn cloudflare_ip(headers: &HeaderMap) -> Result<IpAddr, Rejection> {
    let mut values = headers.get_all("cf-connecting-ip").iter();
    let value = values.next().ok_or(Rejection::InvalidIp)?;

    if values.next().is_some() {
        return Err(Rejection::InvalidIp);
    }

    let ip = value
        .to_str()
        .map_err(|_error| Rejection::InvalidIp)?
        .parse::<IpAddr>()
        .map_err(|_error| Rejection::InvalidIp)?;

    match ip.to_canonical() {
        IpAddr::V4(ip) => Ok(IpAddr::V4(ip)),
        // Ignore host bits so address changes within a /64 share the same limit.
        IpAddr::V6(ip) => Ok(IpAddr::V6(Ipv6Addr::from(u128::from(ip) & (u128::MAX << 64)))),
    }
}

#[derive(Debug)]
enum Rejection {
    InvalidIp,
    Limited(Duration),
    Unavailable,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        let (status, message, retry_after) = match self {
            Self::InvalidIp => (StatusCode::FORBIDDEN, "invalid-client-ip", None),
            Self::Limited(wait) => {
                let seconds = (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1);
                (StatusCode::TOO_MANY_REQUESTS, "rate-limit-exceeded", Some(seconds))
            }
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "rate-limit-unavailable", None),
        };

        let mut response = (status, Json(serde_json::json!({ "error": message }))).into_response();
        response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
            .headers_mut()
            .insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));

        if let Some(seconds) = retry_after {
            response.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }

        response
    }
}

/// Checks the request's allowance before running the next middleware or handler.
pub(crate) async fn enforce(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check_at(request.headers(), Instant::now()) {
        Ok(()) => next.run(request).await,
        Err(rejection) => rejection.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::{
        Router,
        body::{Body, to_bytes},
        middleware,
        routing::get,
    };
    use futures::future::join_all;
    use tokio::sync::Barrier;
    use tower::ServiceExt;

    use super::*;

    fn headers(ip: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", ip.parse().expect("test client IP"));
        headers
    }

    #[test]
    fn local_mode_bypasses_headers_limits_and_storage_entirely() {
        let limiter = RateLimiter::new(RateLimitMode::Local, 1, Duration::from_secs(30));
        let now = Instant::now();

        for header in [HeaderMap::new(), headers("invalid"), headers("192.0.2.1")] {
            for _ in 0..100 {
                limiter.check_at(&header, now).expect("local bypass");
            }
        }

        assert!(limiter.buckets.lock().expect("buckets").clients.is_empty());
    }

    #[test]
    fn cloudflare_requires_exactly_one_plain_ip_without_fallbacks() {
        for value in [
            "",
            "invalid",
            "192.0.2.1, 192.0.2.2",
            " 192.0.2.1",
            "192.0.2.1 ",
            "192.0.2.1:1234",
            "[2001:db8::1]",
            "fe80::1%eth0",
        ] {
            let failure = cloudflare_ip(&headers(value)).expect_err("invalid IP");
            assert!(matches!(failure, Rejection::InvalidIp));
        }

        let mut duplicate = headers("192.0.2.1");
        duplicate.append("cf-connecting-ip", HeaderValue::from_static("192.0.2.1"));
        cloudflare_ip(&duplicate).expect_err("duplicate headers");

        let mut invalid_text = HeaderMap::new();
        invalid_text
            .insert("cf-connecting-ip", HeaderValue::from_bytes(&[0x80]).expect("opaque header"));
        cloudflare_ip(&invalid_text).expect_err("non-ASCII header");

        let mut fallback = HeaderMap::new();
        fallback.insert("x-forwarded-for", HeaderValue::from_static("192.0.2.1"));
        fallback.insert("x-real-ip", HeaderValue::from_static("192.0.2.1"));
        fallback.insert("forwarded", HeaderValue::from_static("for=192.0.2.1"));
        cloudflare_ip(&fallback).expect_err("no forwarding-header fallback");
    }

    #[test]
    fn ipv4_and_mapped_ipv6_share_a_bucket_but_other_ipv4_clients_do_not() {
        let limiter = RateLimiter::new(RateLimitMode::Cloudflare, 1, Duration::from_secs(30));
        let now = Instant::now();

        limiter.check_at(&headers("192.0.2.1"), now).expect("first request");
        let error = limiter.check_at(&headers("::ffff:192.0.2.1"), now).expect_err("same client");
        assert!(matches!(error, Rejection::Limited(_)));
        limiter.check_at(&headers("192.0.2.2"), now).expect("independent client");
    }

    #[test]
    fn ipv6_spelling_and_address_rotation_within_a_64_do_not_reset_the_limit() {
        let limiter = RateLimiter::new(RateLimitMode::Cloudflare, 1, Duration::from_secs(30));
        let now = Instant::now();

        limiter.check_at(&headers("2001:db8:1234:5678::1"), now).expect("first request");
        for ip in
            ["2001:0db8:1234:5678:0000:0000:0000:0001", "2001:db8:1234:5678:ffff:ffff:ffff:ffff"]
        {
            let error = limiter.check_at(&headers(ip), now).expect_err("same /64");
            assert!(matches!(error, Rejection::Limited(_)));
        }

        limiter.check_at(&headers("2001:db8:1234:5679::1"), now).expect("different /64");
    }

    #[test]
    fn token_refill_has_an_exact_boundary_and_denials_do_not_extend_it() {
        let limiter = RateLimiter::new(RateLimitMode::Cloudflare, 2, Duration::from_secs(10));
        let now = Instant::now();
        let client = headers("192.0.2.1");

        limiter.check_at(&client, now).expect("burst token one");
        limiter.check_at(&client, now).expect("burst token two");

        for elapsed in [0, 2, 4, 9] {
            let error =
                limiter.check_at(&client, now + Duration::from_secs(elapsed)).expect_err("limited");
            assert!(
                matches!(error, Rejection::Limited(wait) if wait == Duration::from_secs(10 - elapsed)),
                "unexpected rejection: {error:?}"
            );
        }

        let error = limiter
            .check_at(&client, now + Duration::from_millis(9_999))
            .expect_err("not quite refilled");
        assert!(matches!(error, Rejection::Limited(wait) if wait == Duration::from_millis(1)));
        limiter
            .check_at(&client, now + Duration::from_secs(10))
            .expect("exactly one refilled token");
        limiter.check_at(&client, now + Duration::from_secs(10)).expect_err("no extra token");

        for _ in 0..2 {
            limiter
                .check_at(&client, now + Duration::from_secs(100))
                .expect("fully refilled burst");
        }

        limiter.check_at(&client, now + Duration::from_secs(100)).expect_err("burst still bounded");
    }

    #[test]
    fn capacity_is_bounded_without_resetting_active_client_budgets() {
        let mut limiter = RateLimiter::new(RateLimitMode::Cloudflare, 1, Duration::from_secs(120));
        limiter.max_clients = 2;
        let now = Instant::now();

        limiter.check_at(&headers("192.0.2.1"), now).expect("client one");
        limiter.check_at(&headers("192.0.2.2"), now).expect("client two");
        limiter.check_at(&headers("192.0.2.3"), now).expect_err("capacity bound");
        assert_eq!(limiter.buckets.lock().expect("buckets").clients.len(), 2);
        limiter.check_at(&headers("192.0.2.1"), now).expect_err("existing budget retained");
        limiter
            .check_at(&headers("192.0.2.3"), now + CLEANUP_INTERVAL)
            .expect_err("partially depleted entries stay");
        assert_eq!(limiter.buckets.lock().expect("buckets").clients.len(), 2);
        limiter
            .check_at(&headers("192.0.2.3"), now + Duration::from_secs(120))
            .expect("fully refilled entries reclaimed");
        assert_eq!(limiter.buckets.lock().expect("buckets").clients.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_requests_cannot_overspend_one_client_budget() {
        const CONTENDERS: usize = 64;
        let limiter =
            Arc::new(RateLimiter::new(RateLimitMode::Cloudflare, 5, Duration::from_secs(30)));
        let barrier = Arc::new(Barrier::new(CONTENDERS));
        let now = Instant::now();

        let tasks = (0..CONTENDERS).map(|_| {
            let limiter = Arc::clone(&limiter);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                barrier.wait().await;
                limiter.check_at(&headers("192.0.2.1"), now)
            })
        });

        let results = tokio::time::timeout(Duration::from_secs(5), join_all(tasks))
            .await
            .expect("contenders complete");

        let mut allowed = 0;

        for result in results {
            match result.expect("limiter task") {
                Ok(()) => allowed += 1,
                Err(error) => assert!(matches!(error, Rejection::Limited(_))),
            }
        }

        assert_eq!(allowed, 5);
        assert_eq!(limiter.buckets.lock().expect("buckets").clients.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_new_clients_cannot_overfill_storage() {
        const CONTENDERS: usize = 64;
        let mut limiter = RateLimiter::new(RateLimitMode::Cloudflare, 1, Duration::from_secs(30));
        limiter.max_clients = 8;
        let limiter = Arc::new(limiter);
        let barrier = Arc::new(Barrier::new(CONTENDERS));
        let now = Instant::now();

        let tasks = (1..=CONTENDERS).map(|client| {
            let limiter = Arc::clone(&limiter);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                barrier.wait().await;
                limiter.check_at(&headers(&format!("192.0.2.{client}")), now)
            })
        });

        let results = tokio::time::timeout(Duration::from_secs(5), join_all(tasks))
            .await
            .expect("contenders complete");

        let allowed = results
            .into_iter()
            .filter(|result| result.as_ref().expect("limiter task").is_ok())
            .count();
        assert_eq!(allowed, 8);
        assert_eq!(limiter.buckets.lock().expect("buckets").clients.len(), 8);
    }

    #[tokio::test]
    async fn rejection_responses_are_sanitized_uncacheable_and_round_retry_up() {
        let response = Rejection::Limited(Duration::from_millis(1_001)).into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "2");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
        let body = to_bytes(response.into_body(), 1024).await.expect("error body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");
        assert_eq!(json, serde_json::json!({"error": "rate-limit-exceeded"}));

        let response = Rejection::Limited(Duration::from_nanos(1)).into_response();
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
    }

    #[tokio::test]
    async fn middleware_rejects_before_invoking_the_handler() {
        let calls = Arc::new(AtomicUsize::new(0));
        let handler_calls = Arc::clone(&calls);
        let limiter =
            Arc::new(RateLimiter::new(RateLimitMode::Cloudflare, 1, Duration::from_secs(30)));
        let app = Router::new()
            .route(
                "/",
                get(move || {
                    handler_calls.fetch_add(1, Ordering::Relaxed);
                    async { StatusCode::NO_CONTENT }
                }),
            )
            .route_layer(middleware::from_fn_with_state(limiter, enforce));

        let invalid = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(invalid.status(), StatusCode::FORBIDDEN);
        assert!(!invalid.headers().contains_key(header::RETRY_AFTER));
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        for expected in [StatusCode::NO_CONTENT, StatusCode::TOO_MANY_REQUESTS] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .header("cf-connecting-ip", "192.0.2.1")
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), expected);
        }

        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn invalid_policies_fail_closed_instead_of_allowing_requests() {
        for (burst, interval) in [(0, Duration::from_secs(1)), (1, Duration::ZERO)] {
            let limiter = RateLimiter::new(RateLimitMode::Cloudflare, burst, interval);
            let response = limiter
                .check_at(&headers("192.0.2.1"), Instant::now())
                .expect_err("invalid policy")
                .into_response();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert!(!response.headers().contains_key(header::RETRY_AFTER));
        }
    }
}
