use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rokbattles_desktop_agent::{maintenance_active, now_ms, state_directory};
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
        if maintenance_active()
            .map_err(|_error| "Cannot inspect installation maintenance state.")?
        {
            return Err("Installation maintenance is pending. Complete or repair the update before starting the worker.".to_string());
        }
        let store = self.store().await?;
        let settings =
            store.settings().await.map_err(|_error| "Cannot read background settings.")?;
        self.paused.store(settings.paused, Ordering::SeqCst);
        store
            .set_maintenance_stop(false)
            .await
            .map_err(|_error| "Cannot resume after installation maintenance.")?;
        if !settings.enabled {
            return Ok(());
        }
        // The owner-scoped OS lock is authoritative; heartbeats are only UI status.
        let lease = store.acquire_agent().map_err(|_error| "Cannot inspect background worker.")?;
        if lease.is_none() {
            return Ok(());
        }
        drop(lease);
        spawn_background(settings.capture_opt_in).map_err(|_error| "Background worker executable is unavailable. Reinstall the complete desktop package.".to_string())
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

    pub(crate) async fn capture_opt_in(
        &self,
        app: &AppHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let store = self.store().await?;
        #[cfg(unix)]
        let before = store.settings().await.map_err(|_error| "Cannot read capture settings.")?;
        #[cfg(unix)]
        let protected = if enabled {
            rokbattles_desktop_agent::installed_capture_agent().map_err(
                |_error| "The installed capture companion is not trusted. Repair capture setup.",
            )?
        } else {
            None
        };
        store.set_capture_opt_in(enabled).await.map_err(|_error| "Cannot save capture consent.")?;
        #[cfg(unix)]
        if enabled && !before.capture_opt_in && before.enabled && protected.is_some() {
            // Switch the existing bundled worker once, on explicit opt-in. UI
            // reopen never restarts a healthy active capture connection.
            let lease = self.stop_for_update().await?;
            store
                .set_maintenance_stop(false)
                .await
                .map_err(|_error| "Cannot resume the background worker.")?;
            drop(lease);
            self.launch(app).await?;
        }
        #[cfg(not(unix))]
        let _app = app;
        Ok(())
    }

    pub(crate) async fn stop_for_update(&self) -> Result<AgentLease, String> {
        let store = self.store().await?;
        store
            .set_maintenance_stop(true)
            .await
            .map_err(|_error| "Cannot stop worker for update.")?;
        for _ in 0..150 {
            if let Some(lease) =
                store.acquire_agent().map_err(|_error| "Cannot inspect worker lock.")?
            {
                return Ok(lease);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        store
            .set_maintenance_stop(false)
            .await
            .map_err(|_error| "Cannot restore worker settings.")?;
        Err("Background worker did not stop; update was not installed.".to_string())
    }
}

fn spawn_background(capture_opt_in: bool) -> anyhow::Result<()> {
    let executable = std::env::current_exe()?;
    let parent =
        executable.parent().ok_or_else(|| anyhow::anyhow!("missing executable directory"))?;
    let name =
        if cfg!(windows) { "rokbattles-desktop-agent.exe" } else { "rokbattles-desktop-agent" };
    let bundled = parent.join(name);
    #[cfg(unix)]
    let path = if capture_opt_in {
        rokbattles_desktop_agent::installed_capture_agent()?.unwrap_or(bundled)
    } else {
        bundled
    };
    #[cfg(not(unix))]
    let path = {
        let _capture_opt_in = capture_opt_in;
        bundled
    };
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        anyhow::bail!("invalid worker executable");
    }
    let mut command = Command::new(path);
    // Do not carry loader overrides into the separately authenticated agent.
    for (name, _) in std::env::vars_os() {
        let text = name.to_string_lossy();
        if text.starts_with("LD_") || text.starts_with("DYLD_") || text == "GLIBC_TUNABLES" {
            command.env_remove(name);
        }
    }
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
