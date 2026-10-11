//! Update checks and downloads, with installation coordinated by the watcher manager.
//!
//! Network requests have deadlines and are cancelled when exit is requested.
//! Installation waits for the watcher to finish so its pending uploads can be
//! recovered after restart. Flatpak packages use external updates instead.

use std::{future::Future, time::Duration};

use anyhow::Context;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::watch;

use crate::{watcher::emit_log, watcher_manager::WatcherManager};

fn should_check_for_updates(enabled: bool, is_dev: bool, is_flatpak: bool) -> bool {
    enabled && !is_dev && !is_flatpak
}

async fn bounded_update_request<T>(
    exit: &mut watch::Receiver<bool>,
    operation: impl Future<Output = tauri_plugin_updater::Result<T>>,
    deadline: Duration,
) -> anyhow::Result<Option<T>> {
    tokio::select! {
        biased;

        _ = exit.wait_for(|exiting| *exiting) => Ok(None),
        result = tokio::time::timeout(deadline, operation) => {
            Ok(Some(result.context("Update request timed out")??))
        }
    }
}

async fn install_update_if_available(app: AppHandle) -> anyhow::Result<()> {
    let manager = app.state::<WatcherManager>();
    let mut exit = manager.exit_signal();
    let updater = app.updater_builder().timeout(Duration::from_secs(20)).build()?;
    let Some(Some(mut update)) =
        bounded_update_request(&mut exit, updater.check(), Duration::from_secs(20)).await?
    else {
        return Ok(());
    };

    // Download and verify while scanning continues. Installation may terminate the process
    // on Windows, so it must run only after the manager has joined the watcher.
    update.timeout = Some(Duration::from_secs(120));
    let Some(bytes) = bounded_update_request(
        &mut exit,
        update.download(|_, _| {}, || {}),
        Duration::from_secs(120),
    )
    .await?
    else {
        return Ok(());
    };

    let restart_app = app.clone();
    manager
        .install_update(
            move || {
                update.install(bytes)?;
                Ok(())
            },
            move || restart_app.request_restart(),
        )
        .await
        .map_err(anyhow::Error::msg)?;

    Ok(())
}

pub(crate) async fn maybe_check_for_updates(app: AppHandle, enabled: bool) {
    if !should_check_for_updates(enabled, tauri::is_dev(), crate::is_flatpak()) {
        return;
    }

    if let Err(error) = install_update_if_available(app.clone()).await {
        emit_log(&app, format!("Update check or installation failed: {error:#}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stalled_update_check_has_a_deadline() {
        let (_exit, mut receiver) = watch::channel(false);
        let operation = std::future::pending::<tauri_plugin_updater::Result<()>>();

        let error = bounded_update_request(&mut receiver, operation, Duration::from_millis(20))
            .await
            .expect_err("stalled check must time out");

        assert!(error.to_string().contains("Update request timed out"));
    }

    #[tokio::test]
    async fn exit_cancels_a_pending_check_or_download() {
        let (exit, mut receiver) = watch::channel(false);
        let (started, waiting) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(async move {
            bounded_update_request(
                &mut receiver,
                async {
                    started.send(()).expect("request started");
                    std::future::pending::<tauri_plugin_updater::Result<()>>().await
                },
                Duration::from_secs(120),
            )
            .await
        });
        waiting.await.expect("request is pending");

        exit.send_replace(true);
        let result = tokio::time::timeout(Duration::from_secs(1), request)
            .await
            .expect("exit does not wait for update network timeout")
            .expect("request task")
            .expect("cancelled without a request failure");

        assert!(result.is_none());
    }

    #[test]
    fn checks_for_updates_when_enabled_outside_tauri_dev() {
        assert!(should_check_for_updates(true, false, false));
    }

    #[test]
    fn skips_updates_when_disabled_by_config() {
        assert!(!should_check_for_updates(false, false, false));
    }

    #[test]
    fn skips_updates_during_tauri_dev() {
        assert!(!should_check_for_updates(true, true, false));
    }

    #[test]
    fn skips_appimage_updates_inside_flatpak() {
        assert!(!should_check_for_updates(true, false, true));
    }
}
