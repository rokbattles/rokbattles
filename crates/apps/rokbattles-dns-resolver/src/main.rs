#![forbid(unsafe_code)]

use std::path::PathBuf;

use rokbattles_dns_resolver::{Config, Resolver, router};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    dotenvy::from_path(&dotenv_path).ok();

    let config = Config::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=info,axum=info", env!("CARGO_CRATE_NAME")).into()),
        )
        .init();

    info!(
        bind_addr = %config.bind_addr,
        target_hostname = %config.target_hostname,
        relay_ipv4 = %config.relay_ipv4,
        relay_ipv6 = ?config.relay_ipv6,
        "starting DNS-over-HTTPS resolver"
    );
    let resolver = Resolver::new(config.target_hostname, config.relay_ipv4, config.relay_ipv6);
    let app = router(resolver);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
