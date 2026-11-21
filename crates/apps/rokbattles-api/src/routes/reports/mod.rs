//! Report list and detail endpoints with shared IP-based request budgets.

use std::{sync::Arc, time::Duration};

use axum::{
    Router, middleware,
    routing::{MethodRouter, get},
};

use crate::{
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::AppState,
};

pub mod battle;
pub(crate) mod common;
pub mod duelbattle2;

const LIST_BURST: u32 = 15;
const LIST_REFILL: Duration = Duration::from_secs(4);
const DETAIL_BURST: u32 = 30;
const DETAIL_REFILL: Duration = Duration::from_secs(2);

/// Builds report routes with independent list and detail budgets shared across types.
///
/// Lists allow a burst of 15 and refill at 15 requests per minute. Details allow
/// a burst of 30 and refill at 30 requests per minute, regardless of report ID.
pub(super) fn router(mode: RateLimitMode) -> Router<Arc<AppState>> {
    limited_routes(
        mode,
        get(battle::get),
        get(duelbattle2::get),
        get(battle::get_by_id),
        get(duelbattle2::get_by_id),
    )
}

fn limited_routes<S: Clone + Send + Sync + 'static>(
    mode: RateLimitMode,
    battle_list: MethodRouter<S>,
    arena_list: MethodRouter<S>,
    battle_detail: MethodRouter<S>,
    arena_detail: MethodRouter<S>,
) -> Router<S> {
    let lists = Arc::new(RateLimiter::new(mode, LIST_BURST, LIST_REFILL));
    let details = Arc::new(RateLimiter::new(mode, DETAIL_BURST, DETAIL_REFILL));

    Router::new()
        .route("/reports/battle", battle_list)
        .route("/reports/duelbattle2", arena_list)
        .route_layer(middleware::from_fn_with_state(lists, rate_limit::enforce))
        .merge(
            Router::new()
                .route("/report/battle/{id}", battle_detail)
                .route("/report/duelbattle2/{id}", arena_detail)
                .route_layer(middleware::from_fn_with_state(details, rate_limit::enforce)),
        )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
        response::Response,
    };
    use tower::ServiceExt;

    use super::*;

    const LIST_PATHS: [&str; 2] = ["/reports/battle", "/reports/duelbattle2"];
    const DETAIL_PATHS: [&str; 2] = ["/report/battle/1", "/report/duelbattle2/1"];

    fn limiter_router(mode: RateLimitMode) -> Router {
        limited_routes(
            mode,
            get(|| async { StatusCode::NO_CONTENT }),
            get(|| async { StatusCode::NO_CONTENT }),
            get(|| async { StatusCode::NO_CONTENT }),
            get(|| async { StatusCode::NO_CONTENT }),
        )
    }

    async fn request(app: &Router, path: &str, ip: &str) -> Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("cf-connecting-ip", ip)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
    }

    #[tokio::test]
    async fn list_types_share_fifteen_requests_but_details_have_thirty_more() {
        let app = limiter_router(RateLimitMode::Cloudflare);

        for attempt in 0..15 {
            let path = if attempt % 2 == 0 {
                "/reports/battle?player_id=1"
            } else {
                "/reports/duelbattle2?player_id=2"
            };
            let response = request(&app, path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        for path in LIST_PATHS {
            let response = request(&app, path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(response.headers()[header::RETRY_AFTER], "4");
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let body = to_bytes(response.into_body(), 1024).await.expect("error body");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");
            assert_eq!(json, serde_json::json!({ "error": "rate-limit-exceeded" }));
        }

        for attempt in 0..30 {
            let report_type = if attempt % 2 == 0 { "battle" } else { "duelbattle2" };
            let path = format!("/report/{report_type}/{attempt}");
            let response = request(&app, &path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        for path in ["/report/battle/another-id", "/report/duelbattle2/999"] {
            let response = request(&app, path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(response.headers()[header::RETRY_AFTER], "2");
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
    }

    #[tokio::test]
    async fn exhausting_details_does_not_block_lists() {
        let app = limiter_router(RateLimitMode::Cloudflare);

        for _ in 0..30 {
            let response = request(&app, "/report/battle/1", "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        for path in DETAIL_PATHS {
            let response = request(&app, path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        for path in LIST_PATHS {
            let response = request(&app, path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }
    }

    #[tokio::test]
    async fn report_budgets_are_independent_for_different_client_networks() {
        let app = limiter_router(RateLimitMode::Cloudflare);

        for (path, limit) in [("/reports/battle", 15), ("/report/battle/1", 30)] {
            for _ in 0..limit {
                let response = request(&app, path, "192.0.2.1").await;
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
            }
        }

        for path in LIST_PATHS.into_iter().chain(DETAIL_PATHS) {
            let response = request(&app, path, "192.0.2.1").await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            let response = request(&app, path, "192.0.2.2").await;
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }
    }

    #[tokio::test]
    async fn all_report_routes_require_a_valid_cloudflare_ip() {
        let app = limiter_router(RateLimitMode::Cloudflare);

        for path in LIST_PATHS.into_iter().chain(DETAIL_PATHS) {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            let response = request(&app, path, "invalid").await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn local_mode_bypasses_limits_and_ip_validation_for_all_report_routes() {
        let app = limiter_router(RateLimitMode::Local);

        for path in LIST_PATHS.into_iter().chain(DETAIL_PATHS) {
            for _ in 0..100 {
                let response = app
                    .clone()
                    .oneshot(Request::builder().uri(path).body(Body::empty()).expect("request"))
                    .await
                    .expect("response");
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
            }
        }
    }

    #[tokio::test]
    async fn report_limits_do_not_apply_to_other_routes() {
        let app = limiter_router(RateLimitMode::Cloudflare)
            .route("/health", get(|| async { StatusCode::NO_CONTENT }));

        for _ in 0..100 {
            let response = app
                .clone()
                .oneshot(Request::builder().uri("/health").body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }
    }
}
