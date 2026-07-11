use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

fn should_check_for_updates(enabled: bool, is_dev: bool) -> bool {
    enabled && !is_dev
}

// https://tauri.app/plugin/updater/#checking-for-updates
async fn install_update_if_available(app: AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    println!("download finished");
                },
            )
            .await?;

        println!("update installed");
        app.restart();
    }

    Ok(())
}

pub(crate) async fn maybe_check_for_updates(app: AppHandle, enabled: bool) {
    if !should_check_for_updates(enabled, tauri::is_dev()) {
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
        assert!(should_check_for_updates(true, false));
    }

    #[test]
    fn skips_updates_when_disabled_by_config() {
        assert!(!should_check_for_updates(false, false));
    }

    #[test]
    fn skips_updates_during_tauri_dev() {
        assert!(!should_check_for_updates(true, true));
    }
}
