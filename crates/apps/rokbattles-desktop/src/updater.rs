use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

fn should_check_for_updates(enabled: bool, is_dev: bool, is_flatpak: bool) -> bool {
    enabled && !is_dev && !is_flatpak
}

// https://tauri.app/plugin/updater/#checking-for-updates
async fn install_update_if_available(app: AppHandle) -> anyhow::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        let manager = app.state::<crate::watcher_manager::WatcherManager>();
        let (enabled, lease) = manager.stop_for_update().await.map_err(anyhow::Error::msg)?;
        let installed = update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    println!("download finished");
                },
            )
            .await;
        // Restore desired state; only the relaunched/new UI starts a process.
        manager.store().await.map_err(anyhow::Error::msg)?.set_enabled(enabled).await?;
        if let Err(error) = installed {
            drop(lease);
            if enabled {
                let _restart = manager.launch(&app).await;
            }
            return Err(error.into());
        }
        // Hold the exclusive agent lease through process replacement. A child
        // spawned immediately before maintenance cannot acquire it late.
        let _lease = lease;

        println!("update installed");
        app.restart();
    }

    Ok(())
}

pub(crate) async fn maybe_check_for_updates(app: AppHandle, enabled: bool) {
    if !should_check_for_updates(enabled, tauri::is_dev(), crate::is_flatpak()) {
        return;
    }

    if let Err(e) = install_update_if_available(app).await {
        eprintln!("[rokbattles] update check failed: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::should_check_for_updates;

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
