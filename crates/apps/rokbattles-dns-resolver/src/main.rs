#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::PathBuf;

use rokbattles_dns_resolver::{
    CLOUDFLARE_DOH_FALLBACK_URL, CLOUDFLARE_DOH_PRIMARY_URL, Config, DNS_CHECK_DOMAIN,
    DnsCheckReporter, DoHForwarder, ROCGATE_HOSTNAME, Resolver, router,
};
use tracing::info;

const BIND_ADDRESS: &str = "0.0.0.0:8053";

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
        bind_address = BIND_ADDRESS,
        target_hostname = ROCGATE_HOSTNAME,
        dns_check_domain = DNS_CHECK_DOMAIN,
        gateway = ?config.gateway,
        cloudflare_doh_primary = CLOUDFLARE_DOH_PRIMARY_URL,
        cloudflare_doh_fallback = CLOUDFLARE_DOH_FALLBACK_URL,
        "starting DNS-over-HTTPS resolver"
    );
    let resolver = Resolver::new(config.gateway)?;
    let forwarder = DoHForwarder::new()?;
    let reporter = DnsCheckReporter::new(&config.dns_check_callback_url, &config.dns_check_secret)?;
    let app = router(resolver, forwarder, reporter);

    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
