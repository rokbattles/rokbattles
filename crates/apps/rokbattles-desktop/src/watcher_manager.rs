use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rokbattles_desktop_agent::{now_ms, state_directory};
use rokbattles_desktop_store::{AgentLease, Store};
use tauri::AppHandle;

#[derive(Default)]
pub(crate) struct WatcherManager {
    store: tokio::sync::OnceCell<Arc<Store>>,
    paused: AtomicBool,
    launch: tokio::sync::Mutex<()>,
}

impl WatcherManager {
    pub(crate) async fn store(&self) -> Result<Arc<Store>, String> {
        self.store
            .get_or_try_init(|| async {
                let directory =
                    state_directory().map_err(|_error| "User app-data is unavailable.")?;
                Store::open(&directory)
                    .await
                    .map(Arc::new)
                    .map_err(|_error| "Background state is unavailable or not private.".to_string())
            })
            .await
            .cloned()
    }

    pub(crate) async fn launch(&self, _app: &AppHandle) -> Result<(), String> {
        let _guard = self.launch.lock().await;
        let store = self.store().await?;
        let settings =
            store.settings().await.map_err(|_error| "Cannot read background settings.")?;
        self.paused.store(settings.paused, Ordering::SeqCst);
        if !settings.enabled {
            return Ok(());
        }
        // The owner-scoped OS lock is authoritative; heartbeats are only UI status.
        let lease = store.acquire_agent().map_err(|_error| "Cannot inspect background worker.")?;
        if lease.is_none() {
            return Ok(());
        }
        drop(lease);
        spawn_background().map_err(|_error| "Background worker executable is unavailable. Reinstall the complete desktop package.".to_string())
    }

    pub(crate) async fn start(&self, app: &AppHandle) -> Result<(), String> {
        let store = self.store().await?;
        store.set_enabled(true).await.map_err(|_error| "Cannot enable background worker.")?;
        store.set_paused(false).await.map_err(|_error| "Cannot resume background worker.")?;
        self.paused.store(false, Ordering::SeqCst);
        self.launch(app).await
    }

    pub(crate) async fn stop(&self, _app: &AppHandle) -> Result<(), String> {
        self.store()
            .await?
            .set_paused(true)
            .await
            .map_err(|_error| "Cannot pause background worker.")?;
        self.paused.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub(crate) async fn stop_for_update(&self) -> Result<(bool, AgentLease), String> {
        let store = self.store().await?;
        let enabled =
            store.settings().await.map_err(|_error| "Cannot read worker settings.")?.enabled;
        store.set_enabled(false).await.map_err(|_error| "Cannot stop worker for update.")?;
        for _ in 0..150 {
            if let Some(lease) =
                store.acquire_agent().map_err(|_error| "Cannot inspect worker lock.")?
            {
                return Ok((enabled, lease));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        store.set_enabled(enabled).await.map_err(|_error| "Cannot restore worker settings.")?;
        Err("Background worker did not stop; update was not installed.".to_string())
    }
}

fn spawn_background() -> anyhow::Result<()> {
    let executable = std::env::current_exe()?;
    let parent =
        executable.parent().ok_or_else(|| anyhow::anyhow!("missing executable directory"))?;
    let name =
        if cfg!(windows) { "rokbattles-desktop-agent.exe" } else { "rokbattles-desktop-agent" };
    let path = parent.join(name);
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        anyhow::bail!("invalid worker executable");
    }
    let mut command = Command::new(path);
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000 | 0x00000200); // NO_WINDOW | NEW_PROCESS_GROUP
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    // Reap when it eventually exits. Closing the UI does not signal the agent.
    std::thread::spawn(move || {
        let _status = child.wait();
    });
    Ok(())
}

pub(crate) fn is_alive(status: &rokbattles_desktop_store::Status) -> bool {
    status.running && now_ms().saturating_sub(status.heartbeat_ms) < 30_000
}
