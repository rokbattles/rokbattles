use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

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
    if !enabled {
        return;
    }

    if let Err(e) = install_update_if_available(app).await {
        eprintln!("[rokbattles] update check failed: {}", e);
    }
}
