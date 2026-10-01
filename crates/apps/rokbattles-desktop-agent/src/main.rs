use std::time::Duration;

use rokbattles_desktop_agent::{Agent, state_directory};
use rokbattles_desktop_store::Store;

#[tokio::main]
async fn main() {
    if run().await.is_err() {
        // Static diagnostics only. Paths, mail content and HTTP responses never
        // enter process logs; the UI reads bounded status codes from SQLite.
        eprintln!("ROK Battles background agent stopped: state unavailable");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    if std::env::args_os().len() != 1 {
        anyhow::bail!("agent takes no arguments");
    }
    // Prevent process-local crash dumps before mail or capture buffers exist.
    // Tests exercise an injected policy and do not change the test host's limits.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    rokbattles_capture_ipc::unix::harden_capture_process()?;
    if rokbattles_desktop_agent::maintenance_active()? {
        anyhow::bail!("installation maintenance pending");
    }
    let store = Store::open(&state_directory()?).await?;
    let Some(_lease) = store.acquire_agent()? else {
        return Ok(());
    };
    let mut agent = Agent::new(store)?;
    let result = tokio::select! {
        result = async {
            loop {
                if !agent.tick().await? { break; }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok::<(), anyhow::Error>(())
        } => result,
        _signal = tokio::signal::ctrl_c() => Ok(()),
    };
    agent.close().await;
    result
}
