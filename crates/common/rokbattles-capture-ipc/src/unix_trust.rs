//! Read-only protected Unix installation checks shared by launcher and helper.
use std::{
    fs, io,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
#[cfg(target_os = "linux")]
const AGENT: &str = "/usr/libexec/rokbattles/rokbattles-desktop-agent";
#[cfg(target_os = "macos")]
const AGENT: &str = "/Library/Application Support/ROK Battles/rokbattles-desktop-agent";
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "protected installed agent required")
}
fn protected_attributes(uid: u32, mode: u32) -> bool {
    uid == 0 && mode & 0o022 == 0
}

/// Select only the fixed administrator-installed companion. Missing returns
/// NotFound; a present but untrusted path returns PermissionDenied and must never
/// silently fall back. Running-process signatures remain the helper's job.
pub fn installed_agent_path() -> io::Result<PathBuf> {
    let path = Path::new(AGENT);
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 {
        return Err(denied());
    }
    verify_protected_path(path)?;
    Ok(path.to_owned())
}

/// Root ownership, no group/world writes, no symlink components and (macOS) no
/// extended ACL mutation grants. Benign deny-delete ACLs remain supported.
/// This function reads metadata only; it never repairs permissions or ownership.
pub fn verify_protected_path(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(denied());
    }
    let mut components = path.ancestors().collect::<Vec<_>>();
    components.reverse();
    for (index, component) in components.iter().enumerate() {
        let metadata = fs::symlink_metadata(component)?;
        let directory = index + 1 != components.len() || metadata.is_dir();
        if !protected_attributes(metadata.uid(), metadata.mode())
            || if directory { !metadata.is_dir() } else { !metadata.is_file() }
        {
            return Err(denied());
        }
        let flags =
            libc::O_NOFOLLOW | libc::O_CLOEXEC | if directory { libc::O_DIRECTORY } else { 0 };
        let file = fs::OpenOptions::new().read(true).custom_flags(flags).open(component)?;
        let opened = file.metadata()?;
        if (metadata.dev(), metadata.ino()) != (opened.dev(), opened.ino())
            || !protected_attributes(opened.uid(), opened.mode())
        {
            return Err(denied());
        }
        check_acl(&file)?;
    }
    Ok(())
}
#[cfg(target_os = "linux")]
fn check_acl(_file: &fs::File) -> io::Result<()> {
    // Linux POSIX ACL write grants are constrained by the group-class mode mask,
    // already required to exclude write. Unlike macOS, no independent ACL grant.
    Ok(())
}
#[cfg(target_os = "macos")]
fn check_acl(file: &fs::File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn rb_capture_acl_protected(fd: i32) -> i32;
    }
    // SAFETY: read-only ACL bridge receives our owned live descriptor.
    if unsafe { rb_capture_acl_protected(file.as_raw_fd()) } != 0 {
        return Err(denied());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mode_policy_rejects_foreign_owner_and_any_write_class() {
        assert!(protected_attributes(0, 0o755));
        assert!(protected_attributes(0, 0o644));
        assert!(!protected_attributes(501, 0o755));
        for mode in [0o775, 0o757, 0o777, 0o662] {
            assert!(!protected_attributes(0, mode));
        }
    }
    #[test]
    fn relative_and_symlink_paths_are_rejected_without_installation_changes() {
        verify_protected_path(Path::new("relative-agent")).expect_err("relative");
        let temp = tempfile::tempdir().expect("temporary fixture");
        let path = temp.path().join("agent");
        std::os::unix::fs::symlink("/", &path).expect("fixture symlink");
        verify_protected_path(&path).expect_err("no symlink traversal");
    }
}
