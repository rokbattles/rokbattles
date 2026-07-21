use std::sync::atomic::{AtomicBool, Ordering};

use tauri::AppHandle;

use crate::watcher::{WatcherTask, spawn_watcher};

pub(crate) struct WatcherManager {
    task: tokio::sync::Mutex<Option<WatcherTask>>,
    exiting: AtomicBool,
    paused: AtomicBool,
}

impl Default for WatcherManager {
    fn default() -> Self {
        Self {
            task: tokio::sync::Mutex::new(None),
            exiting: AtomicBool::new(false),
            paused: AtomicBool::new(true),
        }
    }
}

impl WatcherManager {
    pub(crate) async fn start(&self, app: &AppHandle) {
        let mut guard = self.task.lock().await;
        if guard.is_some() {
            self.paused.store(false, Ordering::SeqCst);
            return;
        }

        // Spawn the watcher once per app lifecycle.
        *guard = Some(spawn_watcher(app));
        self.paused.store(false, Ordering::SeqCst);
    }

    pub(crate) async fn stop(&self, app: &AppHandle) {
        let mut guard = self.task.lock().await;
        if let Some(task) = guard.take() {
            task.shutdown(app).await;
        }
        self.paused.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub(crate) fn mark_exit_requested(&self) -> bool {
        !self.exiting.swap(true, Ordering::SeqCst)
    }
}
