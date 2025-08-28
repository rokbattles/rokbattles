pub mod ingress;

use axum::{Router, routing::post};

pub fn router() -> Router {
    Router::new().route("/ingress", post(ingress::ingress))
}
