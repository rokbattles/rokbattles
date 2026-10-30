//! Saves and restores macOS access to watched directories.
//!
//! Settings store security-scoped bookmarks across launches. [`DirectoryAccess`]
//! retains the resolved URLs and access guards for the current process. Each guard
//! balances a successful start call with a stop call when it is dropped.

use std::{collections::BTreeMap, sync::Mutex};

use anyhow::{Context, anyhow};
use objc2::{rc::Retained, runtime::Bool};
use objc2_foundation::{
    NSData, NSString, NSURL, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions,
};
use tauri::{AppHandle, Manager};

use crate::{app_config, watcher::emit_log};

/// Tracks attempted bookmark restorations and retains their access guards.
#[derive(Default)]
pub(crate) struct DirectoryAccess {
    // Keep guards until exit because removed directories can still have reads in flight.
    grants: Mutex<BTreeMap<String, GrantAttempt>>,
}

struct GrantAttempt {
    bookmark: Vec<u8>,
    _scope: Option<SecurityScope>,
}

struct SecurityScope(Retained<NSURL>);

impl Drop for SecurityScope {
    fn drop(&mut self) {
        // SAFETY: This guard is constructed only after a successful matching start call.
        unsafe { self.0.stopAccessingSecurityScopedResource() };
    }
}

fn create_bookmark(url: &NSURL) -> anyhow::Result<Vec<u8>> {
    url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
        NSURLBookmarkCreationOptions::WithSecurityScope
            | NSURLBookmarkCreationOptions::SecurityScopeAllowOnlyReadAccess,
        None,
        None,
    )
    .map(|data| data.to_vec())
    .map_err(|error| anyhow!("Could not save directory access: {error}"))
}

fn resolve_bookmark(data: &[u8]) -> anyhow::Result<(Retained<NSURL>, bool)> {
    let mut stale = Bool::NO;

    // SAFETY: `stale` is a valid writable pointer for the duration of the call.
    let url = unsafe {
        NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
            &NSData::with_bytes(data),
            NSURLBookmarkResolutionOptions::WithSecurityScope
                | NSURLBookmarkResolutionOptions::WithoutUI
                | NSURLBookmarkResolutionOptions::WithoutMounting,
            None,
            &mut stale,
        )
    }
    .map_err(|error| anyhow!("Could not restore directory access: {error}"))?;

    Ok((url, stale.as_bool()))
}

fn start_access(url: Retained<NSURL>) -> Option<SecurityScope> {
    // SAFETY: The URL is retained for the entire access lifetime and a successful
    // start is balanced by SecurityScope::drop. A false return can mean no grant is needed.
    unsafe { url.startAccessingSecurityScopedResource() }.then(|| SecurityScope(url))
}

/// Saves bookmarks for directories the app can already access.
///
/// Reselecting a watched directory retries its saved grant, even if the bookmark
/// bytes have not changed. New directories are restored when the watcher loads them.
///
/// # Errors
///
/// Returns an error if macOS cannot create a bookmark or settings cannot be read
/// or saved.
pub(crate) fn remember(app: &AppHandle, paths: &[String]) -> anyhow::Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    let watched = crate::read_dirs(app)?;
    let mut bookmarks = paths
        .iter()
        .map(|path| {
            let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(path), true);
            create_bookmark(&url).map(|data| (path.clone(), data))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    // A picker can return a canonical spelling of an already watched symlink.
    // Keep its existing display path and attach the grant to that spelling too.
    for path in &watched {
        let selected = paths
            .iter()
            .find(|selected| crate::dir_identity_key(selected) == crate::dir_identity_key(path));

        if let Some(data) = selected.and_then(|selected| bookmarks.get(selected).cloned()) {
            bookmarks.insert(path.clone(), data);
        }
    }

    app.state::<app_config::ConfigStore>().update(|config| {
        config.directory_bookmarks.extend(bookmarks);
    })?;

    restore_selected(app, &app_config::read_config(app)?, paths);

    Ok(())
}

/// Restores saved bookmarks before scanning and retains successful access guards.
///
/// Logs restoration failures and skips subsequent attempts for the same bookmark.
/// A changed bookmark or a new folder selection permits another attempt.
pub(crate) fn restore(app: &AppHandle, config: &app_config::AppConfig) {
    restore_selected(app, config, &[]);
}

fn restore_selected(app: &AppHandle, config: &app_config::AppConfig, selected: &[String]) {
    let access = app.state::<DirectoryAccess>();
    let mut grants = match access.grants.lock() {
        Ok(grants) => grants,
        Err(error) => {
            emit_log(app, format!("Directory access lock failed: {error}"));

            return;
        }
    };

    for path in config.dirs.iter().flatten() {
        let Some(data) = config.directory_bookmarks.get(path) else {
            continue;
        };

        let was_selected = selected
            .iter()
            .any(|selected| crate::dir_identity_key(selected) == crate::dir_identity_key(path));
        let already_attempted = grants.get(path).is_some_and(|grant| grant.bookmark == *data);

        if !was_selected && already_attempted {
            continue;
        }

        let result = resolve_bookmark(data).and_then(|(url, stale)| {
            // Refreshing a stale bookmark requires access to its resolved URL.
            let scope = start_access(url.clone());

            // A moved bookmark must not silently change the watched directory.
            anyhow::ensure!(
                url.path().is_some_and(|resolved| {
                    crate::dir_identity_key(&resolved.to_string()) == crate::dir_identity_key(path)
                }),
                "The saved directory has moved"
            );

            if stale {
                let refreshed =
                    create_bookmark(&url).context("Could not refresh stale directory grant")?;

                app.state::<app_config::ConfigStore>().update(|current| {
                    // Preserve a newer selection saved while this bookmark was resolving.
                    if current.directory_bookmarks.get(path) == Some(data) {
                        current.directory_bookmarks.insert(path.clone(), refreshed);
                    }
                })?;
            }

            Ok(scope)
        });

        let scope = match result {
            Ok(scope) => scope,
            Err(error) => {
                emit_log(
                    app,
                    format!(
                        "Could not restore access to {path:?}: {error:#}. Select this folder again using the Add button; you do not need to remove it first."
                    ),
                );

                None
            }
        };

        // Remember failures too, so a broken grant is not retried every settings poll.
        grants.insert(path.clone(), GrantAttempt { bookmark: data.clone(), _scope: scope });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_directory_bookmark_resolves_in_a_fresh_process() {
        const FIXTURE_ENV: &str = "ROKBATTLES_TEST_BOOKMARK_FIXTURE";

        // The child reads only the persisted fixture, without the parent's NSURL or guard.
        if let Some(fixture) = std::env::var_os(FIXTURE_ENV) {
            let (path, data): (String, Vec<u8>) =
                serde_json::from_slice(&std::fs::read(fixture).expect("read persisted bookmark"))
                    .expect("deserialize persisted bookmark");
            let (resolved, stale) = resolve_bookmark(&data).expect("resolve in a new process");

            assert!(!stale);
            assert_eq!(
                crate::dir_identity_key(&resolved.path().expect("file URL").to_string()),
                crate::dir_identity_key(&path)
            );
            let _scope = start_access(resolved);

            assert_eq!(
                std::fs::read(std::path::Path::new(&path).join("Persistent.Mail.123"))
                    .expect("read source after restoring the bookmark"),
                b"test mail"
            );

            return;
        }

        let dir = tempfile::tempdir().expect("directory");
        let path = dir.path().to_str().expect("UTF-8 path");
        std::fs::write(dir.path().join("Persistent.Mail.123"), b"test mail").expect("mail source");
        let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(path), true);
        let data = create_bookmark(&url).expect("create bookmark");
        drop(url);

        let fixture = dir.path().join("bookmark.json");
        std::fs::write(&fixture, serde_json::to_vec(&(path, data)).expect("serialize bookmark"))
            .expect("persist bookmark");

        let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "directory_access::tests::saved_directory_bookmark_resolves_in_a_fresh_process",
                "--nocapture",
            ])
            .env(FIXTURE_ENV, fixture)
            .output()
            .expect("launch a fresh process");

        assert!(
            output.status.success(),
            "bookmark restoration failed in a fresh process: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn corrupt_bookmark_returns_an_error() {
        resolve_bookmark(b"invalid bookmark").expect_err("invalid data cannot restore a grant");
    }
}
