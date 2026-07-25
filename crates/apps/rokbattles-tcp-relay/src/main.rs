#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{path::PathBuf, sync::Arc};

use rokbattles_tcp_relay::{Config, MailUploader, RuntimeArtifact, serve};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    let _dotenv_result = dotenvy::from_path(&dotenv_path);

    let config = Config::from_env()?;
    let uploader = Some(MailUploader::new(config.relay_token));
    let artifact = Arc::new(RuntimeArtifact::load_default()?);
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME")).into()),
        )
        .init();

    info!(
        bind_addr = %config.bind_addr,
        upstream_addr = %config.upstream_addr,
        carrier_count = artifact.carrier_count(),
        "starting TCP relay"
    );
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    serve(listener, config.upstream_addr, artifact, uploader).await?;

    Ok(())
}
