#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::PathBuf;

use rokbattles_dns_resolver::{Config, DoHForwarder, Resolver, router};
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
        intra_upstream_doh_url = %config.intra_upstream_doh_url,
        "starting DNS-over-HTTPS resolver"
    );
    let resolver = Resolver::new(config.target_hostname, config.relay_ipv4);
    let forwarder = DoHForwarder::new(config.intra_upstream_doh_url)?;
    let app = router(resolver, forwarder);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
