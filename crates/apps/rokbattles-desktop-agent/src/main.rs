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
    // The protected broker authenticates this image and rejects ptraced peers.
    // This only hardens the running agent; no system policy is changed.
    #[cfg(target_os = "linux")]
    rustix::process::set_dumpable_behavior(rustix::process::DumpableBehavior::NotDumpable)?;
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
