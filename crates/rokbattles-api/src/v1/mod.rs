pub mod ingress;

use crate::v1::ingress::IngressState;
use axum::{Router, routing::post};
use tower_http::limit::RequestBodyLimitLayer;

pub fn router() -> Router {
    Router::new()
        .route("/ingress", post(ingress::ingress))
        .layer(RequestBodyLimitLayer::new(2500000)) // 2.5mb
        .with_state(IngressState {
            clamd_addr: std::env::var("CLAMD_ADDR").unwrap_or("clamd:3310".into()),
        })
}
