mod auth;
mod health;
mod v1;

use axum::{Router, routing::get};
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
    http_client: reqwest::Client,
    discord_client_id: String,
    discord_client_secret: String,
    discord_redirect_uri: String,
    auth_redirect_url: String,
    session_cookie_name: String,
    session_cookie_domain: Option<String>,
    session_cookie_secure: bool,
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
        http_client: reqwest::Client::new(),
        discord_client_id: std::env::var("DISCORD_CLIENT_ID")
            .expect("DISCORD_CLIENT_ID environment variable must be set"),
        discord_client_secret: std::env::var("DISCORD_CLIENT_SECRET")
            .expect("DISCORD_CLIENT_SECRET environment variable must be set"),
        discord_redirect_uri: std::env::var("DISCORD_REDIRECT_URI")
            .expect("DISCORD_REDIRECT_URI environment variable must be set"),
        auth_redirect_url: std::env::var("AUTH_REDIRECT_URL").unwrap_or_else(|_| "/".into()),
        session_cookie_name: std::env::var("SESSION_COOKIE_NAME")
            .unwrap_or_else(|_| "rok_session".into()),
        session_cookie_domain: std::env::var("SESSION_COOKIE_DOMAIN").ok().and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }),
        session_cookie_secure: std::env::var("SESSION_COOKIE_SECURE")
            .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
            .unwrap_or(true),
    };

    let router = Router::new()
        .route("/health", get(health::health))
        .nest("/v1", v1::router())
        .nest("/auth", auth::router())
        .with_state(app_state)
        .layer(TimeoutLayer::new(Duration::from_secs(40)))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}
