//! launchd starts before /var/run contains application directories. Create only
//! our fixed directory relative to a pinned, verified system runtime directory.
use std::{
    fs, io,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "protected runtime directory required")
}
pub fn prepare() -> io::Result<()> {
    #[cfg(target_os = "macos")]
    const CANONICAL_RUN: &str = "/private/var/run";
    #[cfg(target_os = "linux")]
    const CANONICAL_RUN: &str = "/run";
    if fs::canonicalize("/var/run")? != Path::new(CANONICAL_RUN) {
        return Err(denied());
    }
    rokbattles_capture_ipc::unix::verify_protected_path(Path::new(CANONICAL_RUN))?;
    for component in Path::new(CANONICAL_RUN).ancestors() {
        let metadata = fs::symlink_metadata(component)?;
        if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(denied());
        }
    }
    let parent = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(CANONICAL_RUN)?;
    // SAFETY: fixed non-escaping child name relative to a pinned root directory.
    let created =
        unsafe { libc::mkdirat(parent.as_raw_fd(), c"rokbattles-capture".as_ptr(), 0o755) } == 0;
    if !created && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fixed child, no symlinks, descriptor owned by File below on success.
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c"rokbattles-capture".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    use std::os::fd::FromRawFd;
    // SAFETY: successful openat transfers ownership of this unique descriptor.
    let directory = unsafe { fs::File::from_raw_fd(descriptor) };
    let metadata = directory.metadata()?;
    if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return Err(denied());
    }
    if created {
        // SAFETY: only the newly created root-owned directory is made searchable;
        // socket contents remain mode0600. Existing permissions are never repaired.
        if unsafe { libc::fchmod(directory.as_raw_fd(), 0o755) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    rokbattles_capture_ipc::unix::verify_protected_path(
        &Path::new(CANONICAL_RUN).join("rokbattles-capture"),
    )?;
    Ok(())
}
