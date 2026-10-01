//! Native code is selected only from fixed administrator-controlled installations.
use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
pub const EXECUTABLE: &str = "/usr/libexec/rokbattles/rokbattles-capture-helper";
#[cfg(target_os = "macos")]
pub const EXECUTABLE: &str = "/Library/PrivilegedHelperTools/com.rokbattles.capture-helper";

fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "protected capture installation required")
}
fn safe_metadata(metadata: &fs::Metadata, directory: bool) -> bool {
    metadata.uid() == 0
        && metadata.mode() & 0o022 == 0
        && if directory { metadata.is_dir() } else { metadata.is_file() }
}
pub(super) fn protected_canonical(path: &Path) -> io::Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    rokbattles_capture_ipc::unix::verify_protected_path(&canonical)?;
    if !canonical.is_absolute() || !safe_metadata(&fs::symlink_metadata(&canonical)?, false) {
        return Err(denied());
    }
    for parent in canonical.ancestors().skip(1) {
        if !safe_metadata(&fs::symlink_metadata(parent)?, true) {
            return Err(denied());
        }
    }
    // Check lexical parents too: a writable ancestor must not be able to swap the
    // original symlink between this check and dlopen. Root-controlled symlinks are OK.
    for parent in path.ancestors().skip(1) {
        let resolved = fs::canonicalize(parent)?;
        if !safe_metadata(&fs::metadata(resolved)?, true) {
            return Err(denied());
        }
    }
    Ok(canonical)
}
fn unsafe_environment(name: &str) -> bool {
    name.starts_with("LD_") || name.starts_with("DYLD_") || name == "GLIBC_TUNABLES"
}

pub fn verify_service() -> io::Result<()> {
    // SAFETY: identity reads have no pointer parameters or side effects.
    if unsafe { libc::geteuid() } != 0 {
        return Err(denied());
    }
    if std::env::vars_os().any(|(name, _)| unsafe_environment(&name.to_string_lossy())) {
        return Err(denied());
    }
    let installed = protected_canonical(Path::new(EXECUTABLE))?;
    if fs::canonicalize(std::env::current_exe()?)? != installed {
        return Err(denied());
    }
    Ok(())
}
pub fn library() -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        // This Apple system image can live in the dyld shared cache with no disk
        // inode. Never fall back to Homebrew or a loader search path.
        for parent in ["/", "/usr", "/usr/lib"] {
            rokbattles_capture_ipc::unix::verify_protected_path(Path::new(parent))?;
            if !safe_metadata(&fs::symlink_metadata(parent)?, true) {
                return Err(denied());
            }
        }
        Ok(PathBuf::from("/usr/lib/libpcap.A.dylib"))
    }
    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "x86_64")]
        const CANDIDATES: &[&str] = &[
            "/usr/lib/x86_64-linux-gnu/libpcap.so.0.8",
            "/usr/lib/x86_64-linux-gnu/libpcap.so.1",
            "/usr/lib64/libpcap.so.1",
            "/usr/lib/libpcap.so.1",
        ];
        #[cfg(target_arch = "aarch64")]
        const CANDIDATES: &[&str] = &[
            "/usr/lib/aarch64-linux-gnu/libpcap.so.0.8",
            "/usr/lib/aarch64-linux-gnu/libpcap.so.1",
            "/usr/lib64/libpcap.so.1",
            "/usr/lib/libpcap.so.1",
        ];
        for candidate in CANDIDATES {
            match protected_canonical(Path::new(candidate)) {
                Ok(path) => return Ok(path),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, "system libpcap is unavailable"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loader_overrides_are_rejected_without_changing_the_environment() {
        for name in [
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "LD_AUDIT",
            "DYLD_INSERT_LIBRARIES",
            "DYLD_LIBRARY_PATH",
            "GLIBC_TUNABLES",
        ] {
            assert!(unsafe_environment(name));
        }
        assert!(!unsafe_environment("LANG"));
    }
}
