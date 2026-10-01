//! Private per-user worker state paths. Never called by a privileged capture host.
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::{
    fs::File,
    path::{Component, Path},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("worker state path is not private")]
    Untrusted,
    #[error("worker state I/O failed")]
    Io(#[from] std::io::Error),
    #[error("worker state security setup failed")]
    Security,
}

/// Create/validate an owner-only directory beneath an existing per-user app-data
/// parent. The application chooses this fixed path; it is not accepted over IPC.
pub fn directory(path: &Path) -> Result<(), Error> {
    if !path.is_absolute() || path.components().any(|part| matches!(part, Component::ParentDir)) {
        return Err(Error::Untrusted);
    }
    #[cfg(unix)]
    return unix::directory(path);
    #[cfg(windows)]
    return windows::directory(path);
    #[cfg(not(any(unix, windows)))]
    Err(Error::Untrusted)
}

/// Open one simple-named, owner-only state file without following symlinks.
pub fn file(directory: &Path, name: &str) -> Result<File, Error> {
    if !simple_name(name) {
        return Err(Error::Untrusted);
    }
    self::directory(directory)?;
    let path = directory.join(name);
    #[cfg(unix)]
    return unix::file(&path);
    #[cfg(windows)]
    return windows::file(&path);
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(Error::Untrusted)
    }
}

/// Verify an existing SQLite sidecar or database before it is handed to SQLite.
/// Missing files are allowed only because the already-private directory protects
/// their later creation; an existing link/reparse/non-private file is rejected.
pub fn existing_file(path: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    return unix::existing_file(path);
    #[cfg(windows)]
    return windows::existing_file(path);
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(Error::Untrusted)
    }
}

#[cfg(unix)]
fn options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    options
}

fn simple_name(name: &str) -> bool {
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.ends_with(['.', ' '])
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return false;
    }
    let base = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return false;
    }
    !(base.len() == 4
        && (base.starts_with("COM") || base.starts_with("LPT"))
        && base.as_bytes().get(3).is_some_and(|byte| (b'1'..=b'9').contains(byte)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_names_cannot_be_streams_devices_or_aliases() {
        for name in [
            "CON",
            "nul.db",
            "COM1",
            "LPT9.txt",
            "file:stream",
            "bad\0name",
            "trailing.",
            "trailing ",
            "../state",
            "sub\\state",
            ".",
            "..",
        ] {
            assert!(!simple_name(name), "{name:?}");
        }
        for name in ["worker.sqlite3", "worker.sqlite3-wal", "worker.sqlite3-shm", "agent.lock"] {
            assert!(simple_name(name));
        }
    }
}
