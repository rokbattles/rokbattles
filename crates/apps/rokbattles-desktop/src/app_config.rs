use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{Context, anyhow};
use serde::{Deserialize, Deserializer, Serialize, de};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CloseBehavior {
    #[default]
    MinimizeToTray,
    Quit,
}

impl<'de> Deserialize<'de> for CloseBehavior {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "ask" | "minimize_to_tray" => Ok(Self::MinimizeToTray),
            "quit" => Ok(Self::Quit),
            value => Err(de::Error::unknown_variant(value, &["minimize_to_tray", "quit"])),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AppConfig {
    // Absence is not permission to cancel recovered uploads, even after saving a preference.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_dirs"
    )]
    pub(crate) dirs: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) close_behavior: CloseBehavior,
    #[serde(default = "default_auto_update")]
    pub(crate) auto_update: bool,
    #[serde(default = "default_auto_start")]
    pub(crate) auto_start: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dirs: None,
            close_behavior: CloseBehavior::MinimizeToTray,
            auto_update: default_auto_update(),
            auto_start: default_auto_start(),
        }
    }
}

fn deserialize_dirs<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    // An omitted field is unknown; an explicitly present field must still be a directory list.
    Vec::<String>::deserialize(deserializer).map(Some)
}

fn default_auto_update() -> bool {
    true
}

fn default_auto_start() -> bool {
    true
}

pub(crate) struct ConfigStore {
    path: PathBuf,
    last_good: Mutex<Option<AppConfig>>,
}

impl ConfigStore {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path, last_good: Mutex::new(None) }
    }

    fn load(&self, last_good: &mut Option<AppConfig>) -> anyhow::Result<AppConfig> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) => {
                let first_run = last_good.is_none()
                    && error.kind() == io::ErrorKind::NotFound
                    && fs::symlink_metadata(&self.path)
                        .is_err_and(|missing| missing.kind() == io::ErrorKind::NotFound);

                if first_run {
                    return Ok(AppConfig::default());
                }

                return Err(error).with_context(|| format!("Failed reading {:?}", self.path));
            }
        };

        let config = parse_config_bytes(&bytes)
            .with_context(|| format!("Invalid settings in {:?}", self.path))?;
        *last_good = Some(config.clone());

        Ok(config)
    }

    pub(crate) fn read(
        &self,
        report_error: impl FnOnce(&anyhow::Error),
    ) -> anyhow::Result<AppConfig> {
        let mut last_good =
            self.last_good.lock().map_err(|error| anyhow!("Settings lock poisoned: {error}"))?;

        match self.load(&mut last_good) {
            Ok(config) => Ok(config),
            Err(error) => {
                let cached = last_good.clone();
                drop(last_good);

                report_error(&error);
                cached.ok_or(error)
            }
        }
    }

    pub(crate) fn update<T>(&self, edit: impl FnOnce(&mut AppConfig) -> T) -> anyhow::Result<T> {
        let mut last_good =
            self.last_good.lock().map_err(|error| anyhow!("Settings lock poisoned: {error}"))?;

        // Never use cached settings to overwrite an unreadable or damaged file.
        let mut config = self.load(&mut last_good)?;
        let result = edit(&mut config);

        self.save(config, &mut last_good)?;

        Ok(result)
    }

    pub(crate) fn update_dirs<T>(
        &self,
        mut edit: impl FnMut(&mut Vec<String>) -> T,
    ) -> anyhow::Result<T> {
        // Resolving paths may block on a disconnected drive. Never do that while
        // holding the settings lock, which is also used by preferences and the watcher.
        for _ in 0..3 {
            let snapshot = {
                let mut last_good = self
                    .last_good
                    .lock()
                    .map_err(|error| anyhow!("Settings lock poisoned: {error}"))?;

                self.load(&mut last_good)?.dirs
            };
            let mut dirs = snapshot.clone().unwrap_or_default();
            let result = edit(&mut dirs);

            let mut last_good = self
                .last_good
                .lock()
                .map_err(|error| anyhow!("Settings lock poisoned: {error}"))?;
            let mut current = self.load(&mut last_good)?;

            if current.dirs != snapshot {
                continue;
            }

            // Preserve unknown directories and preferences changed during path resolution.
            if snapshot.is_some() || !dirs.is_empty() {
                current.dirs = Some(dirs);
            }

            self.save(current, &mut last_good)?;
            return Ok(result);
        }

        Err(anyhow!("Watched directories changed during this operation. Please try again."))
    }

    fn save(&self, config: AppConfig, last_good: &mut Option<AppConfig>) -> anyhow::Result<()> {
        let temporary = prepare_config(&self.path, &config)?;
        temporary.persist(&self.path).context("Failed to replace settings")?;
        *last_good = Some(config);

        sync_config_directory(&self.path)
    }

    pub(crate) fn set_auto_start(
        &self,
        enabled: bool,
        read_enabled: impl FnOnce() -> anyhow::Result<bool>,
        apply: impl Fn(bool) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let mut last_good =
            self.last_good.lock().map_err(|error| anyhow!("Settings lock poisoned: {error}"))?;
        let mut config = self.load(&mut last_good)?;
        config.auto_start = enabled;

        // Finish fallible preparation before changing OS startup registration.
        let temporary = prepare_config(&self.path, &config)?;
        let previous = read_enabled().context("Failed to read OS autostart state")?;

        if let Err(error) = apply(enabled) {
            return Err(restore_auto_start(
                error.context("Failed to apply autostart"),
                previous,
                &apply,
            ));
        }

        if let Err(error) = temporary.persist(&self.path) {
            return Err(restore_auto_start(
                anyhow::Error::new(error).context("Failed to replace settings"),
                previous,
                &apply,
            ));
        }

        *last_good = Some(config);

        // Publication already succeeded. A directory-sync error must not roll back only the OS.
        sync_config_directory(&self.path)
    }
}

fn restore_auto_start(
    error: anyhow::Error,
    previous: bool,
    apply: &impl Fn(bool) -> anyhow::Result<()>,
) -> anyhow::Error {
    match apply(previous) {
        Ok(()) => error.context("Settings were not saved; previous OS autostart state restored"),
        Err(rollback) => anyhow!(
            "Settings were not saved: {error:#}. Restoring OS autostart also failed: {rollback:#}. \
             OS autostart may differ from the saved preference."
        ),
    }
}

fn prepare_config(path: &Path, config: &AppConfig) -> anyhow::Result<tempfile::NamedTempFile> {
    let bytes = serde_json::to_vec_pretty(config).context("Failed to serialize settings")?;
    let dir = path.parent().context("Settings path has no parent directory")?;
    fs::create_dir_all(dir).context("Failed to create settings directory")?;

    let mut temporary = tempfile::NamedTempFile::new_in(dir).context("Create settings temp")?;
    temporary.write_all(&bytes).context("Write settings temp")?;
    temporary.as_file().sync_all().context("Sync settings temp")?;

    // NamedTempFile::persist atomically replaces an existing destination on Windows too.
    Ok(temporary)
}

fn sync_config_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let dir = path.parent().context("Settings path has no parent directory")?;
        fs::File::open(dir)
            .and_then(|directory| directory.sync_all())
            .context("Settings were saved, but syncing the settings directory failed; durability is uncertain")?;
    }

    #[cfg(not(unix))]
    let _ = path;

    Ok(())
}

fn parse_config_bytes(data: &[u8]) -> anyhow::Result<AppConfig> {
    let value: serde_json::Value = serde_json::from_slice(data).context("Invalid JSON")?;

    // Serde also accepts structs as arrays. Only objects may contain named preferences;
    // arrays must be legacy directory lists, including an explicitly empty list.
    if value.is_object() {
        // Use the original bytes so duplicate preference fields still fail validation.
        serde_json::from_slice(data).context("Invalid JSON settings")
    } else {
        let dirs =
            serde_json::from_value::<Vec<String>>(value).context("Invalid JSON directory list")?;

        Ok(AppConfig { dirs: Some(dirs), ..AppConfig::default() })
    }
}

pub(crate) fn read_config(app: &AppHandle) -> anyhow::Result<AppConfig> {
    app.state::<ConfigStore>().read(|error| {
        crate::watcher::emit_log(app, format!("Failed to read settings: {error:#}"));
    })
}

pub(crate) fn update_dirs<T>(
    app: &AppHandle,
    edit: impl FnMut(&mut Vec<String>) -> T,
) -> anyhow::Result<T> {
    app.state::<ConfigStore>().update_dirs(edit)
}

pub(crate) fn get_close_behavior(app: &AppHandle) -> anyhow::Result<CloseBehavior> {
    Ok(read_config(app)?.close_behavior)
}

pub(crate) fn set_close_behavior(app: &AppHandle, behavior: CloseBehavior) -> anyhow::Result<()> {
    app.state::<ConfigStore>().update(|config| config.close_behavior = behavior)
}

pub(crate) fn get_auto_update(app: &AppHandle) -> anyhow::Result<bool> {
    Ok(read_config(app)?.auto_update)
}

pub(crate) fn set_auto_update(app: &AppHandle, enabled: bool) -> anyhow::Result<()> {
    app.state::<ConfigStore>().update(|config| config.auto_update = enabled)
}

pub(crate) fn get_auto_start(app: &AppHandle) -> anyhow::Result<bool> {
    Ok(read_config(app)?.auto_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_requested_settings() {
        let config = AppConfig::default();

        assert!(config.auto_update);
        assert!(config.auto_start);
        assert_eq!(config.close_behavior, CloseBehavior::MinimizeToTray);
        assert!(config.dirs.is_none());
    }

    #[test]
    fn reads_settings_keys_and_ignores_removed_flags() {
        let raw = br#"{"dirs":["/tmp/mail"],"close_behavior":"quit","auto_update":false,"autostart_initialized":true,"experimental_network_introspection":true}"#;
        let config = parse_config_bytes(raw).expect("new config should parse");

        assert_eq!(config.dirs, Some(vec!["/tmp/mail".to_string()]));
        assert_eq!(config.close_behavior, CloseBehavior::Quit);
        assert!(!config.auto_update);
        assert!(config.auto_start);
    }

    #[test]
    fn old_ask_close_behavior_migrates_to_minimize_to_tray() {
        let raw = br#"{"dirs":["/tmp/mail"],"close_behavior":"ask"}"#;
        let config = parse_config_bytes(raw).expect("config without auto_update should parse");

        assert!(config.auto_update);
        assert!(config.auto_start);
        assert_eq!(config.close_behavior, CloseBehavior::MinimizeToTray);
    }

    #[test]
    fn reads_legacy_dirs_only_shape() {
        let raw = br#"["/tmp/one","/tmp/two"]"#;
        let config = parse_config_bytes(raw).expect("legacy dirs-only shape should parse");

        assert_eq!(config.dirs, Some(vec!["/tmp/one".to_string(), "/tmp/two".to_string()]));
        assert_eq!(config.close_behavior, CloseBehavior::MinimizeToTray);
        assert!(config.auto_update);
        assert!(config.auto_start);
    }

    #[test]
    fn explicit_empty_directory_lists_remain_authoritative_in_both_formats() {
        for bytes in [b"[]".as_slice(), br#"{"dirs":[]}"#] {
            let config = parse_config_bytes(bytes).expect("valid empty directory choice");

            assert_eq!(config.dirs, Some(Vec::new()));
        }

        assert!(parse_config_bytes(b"{}").expect("omitted directories").dirs.is_none());
    }

    #[test]
    fn invalid_json_is_rejected() {
        let err = parse_config_bytes(br#"{"dirs":123}"#).expect_err("invalid config must fail");
        assert!(err.to_string().contains("Invalid JSON"));
    }

    #[test]
    fn missing_settings_use_defaults_without_authorizing_directory_removal() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let store = ConfigStore::new(path.clone());

        assert_eq!(store.read(|_| {}).expect("first run"), AppConfig::default());
        assert!(!path.exists(), "reading defaults must not publish an empty directory choice");

        store.update(|config| config.auto_update = false).expect("save first preference");
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("saved file")).expect("saved JSON");

        assert!(saved.get("dirs").is_none());
        assert_eq!(saved["auto_update"], false);
        assert!(ConfigStore::new(path).read(|_| {}).expect("restart").dirs.is_none());
    }

    #[test]
    fn cold_invalid_settings_fail_and_do_not_get_overwritten_by_a_setter() {
        for bytes in [
            b"".as_slice(),
            b" ",
            br#"{"dirs":["#,
            br#"{"dirs":42}"#,
            br#"{"dirs":null}"#,
            b"[[]]",
            br#"[["mail"]]"#,
            b"null",
            br#"{"dirs":["mail"],"dirs":[]}"#,
            br#"{"auto_start":false,"auto_start":true}"#,
        ] {
            let dir = tempfile::tempdir().expect("settings directory");
            let path = dir.path().join("config.json");
            fs::write(&path, bytes).expect("invalid settings");
            let store = ConfigStore::new(path.clone());
            let reported = std::cell::Cell::new(false);

            store.read(|_| reported.set(true)).expect_err("invalid settings");
            assert!(reported.get());
            store
                .update(|config| config.auto_update = false)
                .expect_err("must not overwrite invalid settings");
            assert_eq!(fs::read(path).expect("original file retained"), bytes);
        }
    }

    #[test]
    fn failed_refresh_keeps_last_good_values_but_blocks_writes_until_repaired() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let store = ConfigStore::new(path.clone());
        store
            .update(|config| {
                config.dirs = Some(vec!["mail".to_string()]);
                config.auto_update = false;
                config.auto_start = false;
                config.close_behavior = CloseBehavior::Quit;
            })
            .expect("initial settings");
        let expected = store.read(|_| {}).expect("valid settings");

        for failure in ["empty", "truncated", "unreadable", "missing"] {
            fs::remove_file(&path).expect("remove fixture settings");

            match failure {
                "empty" => fs::write(&path, b"").expect("empty settings"),
                "truncated" => fs::write(&path, br#"{"dirs":["#).expect("truncated settings"),
                "unreadable" => fs::create_dir(&path).expect("unreadable settings path"),
                "missing" => {}
                value => panic!("unexpected test case: {value}"),
            }

            let reported = std::cell::Cell::new(false);
            let current = store.read(|_| reported.set(true)).expect("last good settings");

            assert!(reported.get(), "{failure} must be reported");
            assert_eq!(current, expected);
            store
                .update(|config| config.auto_start = true)
                .expect_err("must not overwrite unavailable settings");

            if path.is_dir() {
                fs::remove_dir(&path).expect("remove unreadable fixture");
            }

            fs::write(&path, serde_json::to_vec(&expected).expect("valid JSON"))
                .expect("repair settings");
        }

        store.update(|config| config.auto_start = true).expect("writes resume after repair");
        assert!(store.read(|_| {}).expect("repaired settings").auto_start);
    }

    #[test]
    fn saving_a_legacy_directory_list_preserves_its_paths_and_defaults() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        fs::write(&path, br#"["/tmp/one","/tmp/two"]"#).expect("legacy settings");
        let store = ConfigStore::new(path.clone());

        store.update(|config| config.auto_update = false).expect("migrate on save");
        let config = ConfigStore::new(path).read(|_| {}).expect("reload migrated settings");

        assert_eq!(config.dirs, Some(vec!["/tmp/one".to_string(), "/tmp/two".to_string()]));
        assert!(!config.auto_update);
        assert!(config.auto_start);
        assert_eq!(config.close_behavior, CloseBehavior::MinimizeToTray);
    }

    #[test]
    fn concurrent_preference_and_directory_edits_preserve_every_successful_change() {
        let dir = tempfile::tempdir().expect("settings directory");
        let store = ConfigStore::new(dir.path().join("config.json"));
        let start = std::sync::Barrier::new(5);

        std::thread::scope(|scope| {
            for field in 0..4 {
                let store = &store;
                let start = &start;

                scope.spawn(move || {
                    start.wait();
                    store
                        .update(|config| match field {
                            0 => config.auto_update = false,
                            1 => config.close_behavior = CloseBehavior::Quit,
                            2 => config.dirs.get_or_insert_default().push("first".to_string()),
                            3 => config.dirs.get_or_insert_default().push("second".to_string()),
                            value => panic!("unexpected test case: {value}"),
                        })
                        .expect("concurrent edit");
                });
            }

            start.wait();
        });

        let config = store.read(|_| {}).expect("all changes");
        let mut dirs = config.dirs.expect("saved directories");
        dirs.sort();

        assert!(!config.auto_update);
        assert_eq!(config.close_behavior, CloseBehavior::Quit);
        assert_eq!(dirs, vec!["first", "second"]);
    }

    #[test]
    fn unlocked_readers_see_only_complete_old_or_new_settings_during_publication() {
        use std::sync::{
            Barrier,
            atomic::{AtomicBool, Ordering},
        };

        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let store = ConfigStore::new(path.clone());
        let old = AppConfig {
            dirs: Some((0..500).map(|id| format!("old/{id}")).collect()),
            ..AppConfig::default()
        };
        let new = AppConfig {
            dirs: Some((0..500).map(|id| format!("new/{id}")).collect()),
            auto_update: false,
            close_behavior: CloseBehavior::Quit,
            ..AppConfig::default()
        };
        store.update(|config| *config = old.clone()).expect("initial publication");
        let start = Barrier::new(2);
        let finished = AtomicBool::new(false);

        std::thread::scope(|scope| {
            let reader = scope.spawn(|| {
                start.wait();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                let mut reads = 0;

                while !finished.load(Ordering::Acquire) || reads < 100 {
                    assert!(std::time::Instant::now() < deadline, "writer must complete");
                    let bytes = fs::read(&path).expect("destination always exists");
                    let config = parse_config_bytes(&bytes).expect("complete settings JSON");
                    assert!(config == old || config == new, "reader must see one whole snapshot");
                    reads += 1;
                }
            });

            start.wait();

            for version in 0..40 {
                let next = if version % 2 == 0 { &new } else { &old };
                store.update(|config| *config = next.clone()).expect("publish settings");
            }

            finished.store(true, Ordering::Release);
            reader.join().expect("reader never observed missing or partial settings");
        });
    }

    #[test]
    fn abandoned_preparation_and_failed_replacement_preserve_the_previous_file() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let previous = br#"{"dirs":["mail"],"auto_update":false}"#;
        fs::write(&path, previous).expect("previous file");

        let prepared = prepare_config(&path, &AppConfig::default()).expect("prepare new file");
        drop(prepared);

        assert_eq!(fs::read(&path).expect("previous file"), previous);
        assert_eq!(fs::read_dir(dir.path()).expect("directory").count(), 1);

        let prepared = prepare_config(&path, &AppConfig::default()).expect("prepare again");
        fs::remove_file(prepared.path()).expect("inject missing temp at replacement");

        prepared.persist(&path).expect_err("replacement must fail without its source");
        assert_eq!(fs::read(&path).expect("previous file survives replacement failure"), previous);
    }

    #[test]
    fn autostart_apply_failure_restores_actual_os_state_without_publishing() {
        use std::cell::{Cell, RefCell};

        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let previous = br#"{"dirs":["mail"],"auto_start":false}"#;
        fs::write(&path, previous).expect("previous settings");
        let store = ConfigStore::new(path.clone());
        // The OS can differ from the saved preference before this command starts.
        let os_enabled = Cell::new(true);
        let calls = RefCell::new(Vec::new());

        let error = store
            .set_auto_start(
                false,
                || Ok(os_enabled.get()),
                |enabled| {
                    calls.borrow_mut().push(enabled);
                    os_enabled.set(enabled);

                    if !enabled {
                        anyhow::bail!("registration changed but command failed");
                    }

                    Ok(())
                },
            )
            .expect_err("failed OS update");

        assert!(format!("{error:#}").contains("previous OS autostart state restored"));
        assert_eq!(*calls.borrow(), vec![false, true]);
        assert!(os_enabled.get());
        assert_eq!(fs::read(&path).expect("previous settings"), previous);
    }

    #[test]
    fn autostart_publication_failure_restores_os_state_and_keeps_saved_settings() {
        use std::cell::Cell;

        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let previous = br#"{"dirs":["mail"],"auto_start":false}"#;
        fs::write(&path, previous).expect("previous settings");
        let store = ConfigStore::new(path.clone());
        let os_enabled = Cell::new(false);

        let error = store
            .set_auto_start(
                true,
                || Ok(os_enabled.get()),
                |enabled| {
                    os_enabled.set(enabled);

                    if enabled {
                        for entry in fs::read_dir(dir.path())? {
                            let candidate = entry?.path();
                            if candidate != path {
                                fs::remove_file(candidate)?;
                            }
                        }
                    }

                    Ok(())
                },
            )
            .expect_err("replacement fault");

        assert!(format!("{error:#}").contains("Failed to replace settings"));
        assert!(!os_enabled.get());
        assert_eq!(fs::read(path).expect("previous settings"), previous);
        assert!(!store.read(|_| {}).expect("previous snapshot").auto_start);
    }

    #[test]
    fn autostart_reports_partial_failure_when_restoring_os_state_also_fails() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let previous = br#"{"auto_start":false}"#;
        fs::write(&path, previous).expect("previous settings");
        let store = ConfigStore::new(path.clone());

        let error = store
            .set_auto_start(
                true,
                || Ok(false),
                |enabled| {
                    anyhow::bail!(if enabled { "enable failed" } else { "rollback failed" });
                },
            )
            .expect_err("partial failure");
        let message = format!("{error:#}");

        assert!(message.contains("enable failed"));
        assert!(message.contains("rollback failed"));
        assert!(message.contains("OS autostart may differ from the saved preference"));
        assert_eq!(fs::read(path).expect("original settings"), previous);
    }

    #[test]
    fn autostart_read_failure_does_not_change_os_or_saved_settings() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let previous = br#"{"auto_start":false}"#;
        fs::write(&path, previous).expect("previous settings");
        let store = ConfigStore::new(path.clone());

        assert!(
            store
                .set_auto_start(
                    true,
                    || anyhow::bail!("query failed"),
                    |_| {
                        panic!("must not apply autostart when its previous state is unknown");
                    }
                )
                .is_err()
        );
        assert_eq!(fs::read(path).expect("original settings"), previous);
        assert_eq!(fs::read_dir(dir.path()).expect("directory").count(), 1);
    }

    #[test]
    fn successful_autostart_change_agrees_with_persisted_settings() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let store = ConfigStore::new(path.clone());
        let os_enabled = std::cell::Cell::new(true);

        store
            .set_auto_start(
                false,
                || Ok(os_enabled.get()),
                |enabled| {
                    os_enabled.set(enabled);
                    Ok(())
                },
            )
            .expect("successful change");

        let config = ConfigStore::new(path).read(|_| {}).expect("persisted settings");

        assert!(!os_enabled.get());
        assert!(!config.auto_start);
        assert!(config.dirs.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn settings_write_failure_preserves_old_file_and_never_changes_autostart() {
        use std::os::unix::fs::PermissionsExt;

        // Root bypasses directory permissions, so this OS fault requires an unprivileged user.
        // SAFETY: geteuid takes no arguments and has no preconditions.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let previous = br#"{"dirs":["mail"],"auto_start":false}"#;
        fs::write(&path, previous).expect("previous settings");
        let store = ConfigStore::new(path.clone());
        let applied = std::cell::Cell::new(false);
        let permissions = fs::metadata(dir.path()).expect("directory metadata").permissions();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500))
            .expect("deny creating a settings temp file");

        let preference_result = store.update(|config| config.auto_update = false);
        let autostart_result = store.set_auto_start(
            true,
            || Ok(false),
            |_| {
                applied.set(true);
                Ok(())
            },
        );
        fs::set_permissions(dir.path(), permissions).expect("restore directory permissions");

        assert!(
            format!("{:#}", preference_result.expect_err("failed preference write"))
                .contains("Create settings temp")
        );
        autostart_result.expect_err("failed preparation");
        assert!(!applied.get());
        assert_eq!(fs::read(path).expect("original settings"), previous);
    }

    #[cfg(unix)]
    #[test]
    fn post_publication_sync_failure_reports_committed_settings_without_rolling_back_os() {
        use std::os::unix::fs::PermissionsExt;

        // SAFETY: geteuid takes no arguments and has no preconditions.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let store = ConfigStore::new(path.clone());
        let os_enabled = std::cell::Cell::new(false);
        let permissions = fs::metadata(dir.path()).expect("directory metadata").permissions();

        let result = store.set_auto_start(
            true,
            || Ok(false),
            |enabled| {
                os_enabled.set(enabled);
                // Rename needs write/execute permission, but opening the directory for sync needs read.
                fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o300))?;

                Ok(())
            },
        );
        fs::set_permissions(dir.path(), permissions).expect("restore directory permissions");
        let error = result.expect_err("directory sync failed after replacement");
        let persisted = ConfigStore::new(path).read(|_| {}).expect("published settings");

        assert!(format!("{error:#}").contains("Settings were saved"));
        assert!(os_enabled.get(), "must not roll back the OS after publication");
        assert!(persisted.auto_start);
        assert_eq!(store.read(|_| {}).expect("current settings"), persisted);
    }

    #[cfg(unix)]
    #[test]
    fn dangling_settings_symlink_is_not_first_run_absence() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let target = dir.path().join("unavailable.json");
        std::os::unix::fs::symlink(&target, &path).expect("dangling settings link");
        let store = ConfigStore::new(path.clone());

        store.read(|_| {}).expect_err("broken symlink must not be read as defaults");
        store
            .update(|config| config.auto_update = false)
            .expect_err("must not overwrite invalid settings");
        assert_eq!(fs::read_link(path).expect("preserved settings link"), target);
    }

    #[test]
    fn empty_directory_commands_do_not_invent_an_explicit_removal() {
        let dir = tempfile::tempdir().expect("settings directory");
        let path = dir.path().join("config.json");
        let store = ConfigStore::new(path.clone());

        store.update_dirs(|_| {}).expect("empty add request");
        store.update_dirs(|dirs| dirs.retain(|dir| dir != "missing")).expect("remove unknown path");

        assert!(ConfigStore::new(path).read(|_| {}).expect("saved settings").dirs.is_none());

        store.update_dirs(|dirs| dirs.push("mail".to_string())).expect("add a directory");
        store.update_dirs(|dirs| dirs.retain(|dir| dir != "mail")).expect("remove last directory");

        assert_eq!(store.read(|_| {}).expect("deliberate removal").dirs, Some(Vec::new()));
    }
    #[test]
    fn slow_directory_resolution_preserves_concurrent_preferences_and_directory_choices() {
        use std::{
            sync::{Arc, mpsc},
            time::Duration,
        };
        let temp = tempfile::tempdir().expect("settings");
        let store = Arc::new(ConfigStore::new(temp.path().join("config.json")));
        store.update_dirs(|dirs| dirs.push("original".to_string())).expect("initial directory");
        let (started, ready) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let editing = store.clone();
        let worker = std::thread::spawn(move || {
            let mut first = true;
            editing.update_dirs(|dirs| {
                if first {
                    first = false;
                    started.send(()).expect("started");
                    blocked.recv_timeout(Duration::from_secs(5)).expect("resolution released");
                }
                dirs.push("discovered".to_string());
            })
        });
        ready.recv_timeout(Duration::from_secs(2)).expect("resolution started");

        store.update(|config| config.auto_update = false).expect("preference remains responsive");
        store
            .update_dirs(|dirs| {
                dirs.retain(|path| path != "original");
                dirs.push("manual".to_string());
            })
            .expect("manual change remains responsive");
        release.send(()).expect("release resolution");
        worker.join().expect("worker").expect("merge");

        let config = store.read(|_| {}).expect("settings");
        assert!(!config.auto_update);
        assert_eq!(config.dirs, Some(vec!["manual".to_string(), "discovered".to_string()]));
    }

    #[test]
    fn continuous_directory_conflicts_stop_without_overwriting_new_choices() {
        let temp = tempfile::tempdir().expect("settings");
        let store = ConfigStore::new(temp.path().join("config.json"));
        let mut attempts = 0;
        let result = store.update_dirs(|dirs| {
            attempts += 1;
            store
                .update(|config| config.dirs = Some(vec![format!("concurrent-{attempts}")]))
                .expect("concurrent update");
            dirs.push("stale".to_string());
        });

        assert!(result.is_err());
        assert_eq!(attempts, 3);
        assert_eq!(
            store.read(|_| {}).expect("settings").dirs,
            Some(vec!["concurrent-3".to_string()])
        );
    }
}
