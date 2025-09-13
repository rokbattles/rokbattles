mod health;
mod v1;

use axum::{Router, routing::get};
use mongodb::{IndexModel, bson::Document, bson::doc, options::IndexOptions};
use std::{net::SocketAddr, time::Duration};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{debug, warn};
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

    if let Err(e) = create_indexes(&db).await {
        warn!("failed to create indexes: {}", e);
    }

    let app_state = AppState {
        db,
        clamd_addr: std::env::var("CLAMD_ADDR").unwrap_or("clamd:3310".into()),
    };

    let router = Router::new()
        .route("/health", get(health::health))
        .nest("/v1", v1::router())
        .with_state(app_state)
        .layer(TimeoutLayer::new(Duration::from_secs(40)))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}

async fn create_indexes(db: &mongodb::Database) -> anyhow::Result<()> {
    let mails = db.collection::<Document>("mails");
    let battle = db.collection::<Document>("battleReports");

    mails
        .create_indexes(vec![
            IndexModel::builder()
                .keys(doc! { "mail.hash": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "status": 1, "mail.time": 1 })
                .build(),
        ])
        .await?;

    battle
        .create_indexes(
            vec![
                IndexModel::builder()
                    .keys(doc! { "metadata.hash": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .build(),
                    )
                    .build(),
                IndexModel::builder()
                    .keys(doc! { "metadata.parentHash": 1, "report.metadata.start_date": 1, "metadata.hash": 1 })
                    .build(),
                IndexModel::builder()
                    .keys(doc! { "report.metadata.start_date": -1 })
                    .build(),
                IndexModel::builder()
                    .keys(doc! { "report.self.player_id": 1, "report.metadata.start_date": -1 })
                    .build(),
                IndexModel::builder()
                    .keys(doc! { "report.enemy.player_id": 1, "report.metadata.start_date": -1 })
                    .build(),
                IndexModel::builder()
                    .keys(doc! { "report.metadata.is_kvk": 1, "report.metadata.start_date": -1 })
                    .build(),
                IndexModel::builder()
                    .keys(doc! { "report.metadata.email_role": 1, "report.metadata.start_date": -1 })
                    .build(),
            ],
        )
        .await?;

    Ok(())
}
