//! Run privileged startup, then collect and upload with the worker identity.
//!
//! Startup order is part of the service contract: bind NFLOG before changing
//! rules, remove privileges before parsing payloads, and start threads last.
//! Fatal capture errors terminate the process for systemd to restart; the kernel
//! keeps forwarding with its installed rules and conntrack entries.

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

    // Read the token only after disabling core files; resolve and validate all
    // privileged startup inputs before creating the runtime or worker threads.
    let config = Config::from_env()?;
    let upstream = service::resolve(&config)?;
    let (uid, gid) = service::worker_identity()?;
    let spool_path = PathBuf::from(
        std::env::var("STATE_DIRECTORY")
            .unwrap_or_else(|_| "/var/lib/rokbattles-nat-gateway".into()),
    );
    service::validate_spool(&spool_path, uid)?;

    // Bind the exclusive capture group before installing rules that emit copies.
    // Kernel forwarding outlives this descriptor and the process.
    let capture = Capture::open()?;
    service::install(&config, upstream)?;

    // Errors after an identity change are fatal: partial privilege changes must
    // not fall through to payload decoding with an unexpected identity.
    service::drop_privileges(uid, gid)?;

    let artifact = RuntimeArtifact::load_default()?;
    let spool = Spool::open(spool_path)?;
    let uploader = MailUploader::new(config.relay_token)?;
    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async move {
        let uploads = spool.clone();
        let _uploader = tokio::spawn(async move { uploads.upload_loop(uploader).await });

        // recvfrom and protocol decoding are synchronous; keep them off Tokio's
        // async workers so upload progress is independent of capture throughput.
        let worker = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut observer = Observer::new(&artifact, spool.spawn_writer());
            let mut buffer = vec![0; 131072];
            tracing::info!(%upstream, "gateway ready; kernel forwarding is independent of this process");

            loop {
                match capture.receive(&mut buffer) {
                    Ok(count) => {
                        let datagram = buffer
                            .get(..count)
                            .ok_or_else(|| anyhow::anyhow!("invalid capture length"))?;

                        match records(datagram) {
                            Ok(records) => {
                                for record in records {
                                    observer.record(record);
                                }
                            }
                            Err(reason) => observer.lost(reason),
                        }
                    }
                    Err(error) if error.raw_os_error() == Some(libc::ENOBUFS) => {
                        observer.lost("NFLOG receive buffer overflow");
                    }
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
