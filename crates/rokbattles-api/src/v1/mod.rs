pub mod ingress;

use crate::v1::ingress::IngressState;
use axum::{Router, routing::post};

pub fn router() -> Router {
    Router::new()
        .route("/ingress", post(ingress::ingress))
        .with_state(IngressState {
            clamd_addr: std::env::var("CLAMD_ADDR").unwrap_or("clamd:3310".into()),
        })
}
