//! Unprivileged, UI-independent background agent. No driver opens or elevation.
#![forbid(unsafe_code)]

mod agent;
mod capture;
pub mod capture_http;
pub mod mailcache;
pub use agent::Agent;
mod upload;

use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

pub fn state_directory() -> anyhow::Result<PathBuf> {
    let parent =
        dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("user app-data unavailable"))?;
    Ok(parent.join("com.rokbattles.tauri").join("worker"))
}

pub fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

/// Maintenance is installer-owned. Presence or inability to inspect the marker
/// blocks launch; capture readiness itself never blocks mailcache fallback.
pub fn maintenance_active() -> anyhow::Result<bool> {
    #[cfg(windows)]
    {
        let image = std::env::current_exe()?;
        let directory =
            image.parent().ok_or_else(|| anyhow::anyhow!("missing executable directory"))?;
        maintenance_at(directory)
    }
    #[cfg(not(windows))]
    Ok(false)
}

#[cfg(any(windows, test))]
fn maintenance_at(directory: &std::path::Path) -> anyhow::Result<bool> {
    match std::fs::symlink_metadata(directory.join(".capture-maintenance")) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn maintenance_presence_blocks_even_when_marker_is_a_dangling_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        assert!(!super::maintenance_at(temp.path()).expect("absent"));
        #[cfg(unix)]
        std::os::unix::fs::symlink("missing", temp.path().join(".capture-maintenance"))
            .expect("dangling marker");
        #[cfg(windows)]
        std::fs::write(temp.path().join(".capture-maintenance"), b"").expect("marker");
        assert!(super::maintenance_at(temp.path()).expect("present"));
    }
}
