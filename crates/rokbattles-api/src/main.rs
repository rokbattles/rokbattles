mod v1;

use axum::Router;
use std::{net::SocketAddr, time::Duration};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[derive(Clone)]
struct AppState {
    db: mongodb::Database,
    clamd_addr: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mongo_uri = std::env::var("MONGO_URI").expect("MONGO_URI environment variable must be set");
    let client = mongodb::Client::with_uri_str(&mongo_uri)
        .await
        .expect("failed to create MongoDB client");
    let db = client
        .default_database()
        .expect("MONGO_URI environment variable must include a database name");
    debug!("connected to MongoDB, using database '{}'", db.name());

    let app_state = AppState {
        db,
        clamd_addr: std::env::var("CLAMD_ADDR").unwrap_or("clamd:3310".into()),
    };

    let router = Router::new()
        .nest("/v1", v1::router())
        .with_state(app_state)
        .layer(TimeoutLayer::new(Duration::from_secs(40)))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}
