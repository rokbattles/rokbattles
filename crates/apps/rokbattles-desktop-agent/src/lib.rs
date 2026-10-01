//! Unprivileged, UI-independent background agent. No driver opens or elevation.
#![forbid(unsafe_code)]

mod agent;
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
