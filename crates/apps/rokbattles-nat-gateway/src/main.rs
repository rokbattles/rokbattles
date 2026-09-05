//! One-service startup: bind capture, install NAT, drop root, observe and upload.

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use rokbattles_gateway_protocol::{MailUploader, RuntimeArtifact};
#[cfg(target_os = "linux")]
use rokbattles_nat_gateway::{
    capture::{Capture, records},
    config::Config,
    observer::Observer,
    service,
    spool::Spool,
};

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    service::disable_core_dumps()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let config = Config::from_env()?;
    let upstream = service::resolve(&config)?;
    let (uid, gid) = service::worker_identity()?;
    let spool_path = PathBuf::from(
        std::env::var("STATE_DIRECTORY")
            .unwrap_or_else(|_| "/var/lib/rokbattles-nat-gateway".into()),
    );
    service::validate_spool(&spool_path, uid)?;
    let capture = Capture::open()?;
    service::install(&config, upstream)?;
    service::drop_privileges(uid, gid)?;
    let artifact = RuntimeArtifact::load_default()?;
    let spool = Spool::open(spool_path)?;
    let uploader = MailUploader::new(config.relay_token)?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let uploads = spool.clone();
        let _uploader = tokio::spawn(async move { uploads.upload_loop(uploader).await });
        let worker = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut observer = Observer::new(&artifact, spool.spawn_writer());
            let mut buffer = vec![0; 131072];
            tracing::info!(%upstream, "gateway ready; kernel forwarding is independent of this process");
            loop {
                match capture.receive(&mut buffer) {
                    Ok(count) => match records(buffer.get(..count).ok_or_else(|| anyhow::anyhow!("invalid capture length"))?) {
                        Ok(records) => for record in records { observer.record(record); },
                        Err(reason) => observer.lost(reason),
                    },
                    Err(error) if error.raw_os_error() == Some(libc::ENOBUFS) => observer.lost("NFLOG receive buffer overflow"),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error.into()),
                }
            }
        });
        worker.await?
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("rokbattles-nat-gateway requires Linux");
    std::process::exit(1);
}
