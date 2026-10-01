//! Fixed root-owned Unix socket directory and kernel peer credentials.
//! The directory must be provisioned separately; runtime never creates/chmods it.
use std::{
    fs, io,
    os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, MetadataExt},
    },
    path::{Path, PathBuf},
};
use tokio::net::{UnixListener, UnixStream};

const DIRECTORY: &str = "/var/run/rokbattles-capture";
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "capture socket identity rejected")
}
fn endpoint(uid: u32) -> PathBuf {
    Path::new(DIRECTORY).join(format!("{uid}.sock"))
}

fn check_directory(path: &Path, owner: u32) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o022 != 0 {
        return Err(denied());
    }
    Ok(())
}

/// Connect only to the fixed endpoint owned by this user, then require root as
/// the peer. A user's replacement socket cannot impersonate the helper.
pub async fn connect_current_user() -> io::Result<UnixStream> {
    // SAFETY: geteuid has no pointer parameters or side effects.
    let uid = unsafe { libc::geteuid() };
    check_directory(Path::new(DIRECTORY), 0)?;
    let path = endpoint(uid);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_socket() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return Err(denied());
    }
    let stream = UnixStream::connect(path).await?;
    authenticate(&stream, 0)?;
    Ok(stream)
}

/// Kernel credential check on every accepted connection, independent of pathname permissions.
pub fn authenticate(stream: &UnixStream, expected_uid: u32) -> io::Result<()> {
    if peer_uid(stream)? != expected_uid {
        return Err(denied());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials: libc::ucred = unsafe_zero_credentials();
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: valid stream descriptor, correctly sized writable credential struct and length.
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if length as usize != std::mem::size_of::<libc::ucred>() {
        return Err(denied());
    }
    Ok(credentials.uid)
}
#[cfg(target_os = "linux")]
fn unsafe_zero_credentials() -> libc::ucred {
    libc::ucred { pid: 0, uid: 0, gid: 0 }
}

#[cfg(target_os = "macos")]
fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: valid descriptor and two writable uid/gid outputs.
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(uid)
}

/// Requires the root-owned, non-writable runtime directory to exist already.
/// Refuses stale/preexisting sockets instead of deleting another listener.
pub fn bind_for_user(uid: u32) -> io::Result<UnixListener> {
    // SAFETY: geteuid only reads the current effective identity.
    if unsafe { libc::geteuid() } != 0 || uid == 0 {
        return Err(denied());
    }
    check_directory(Path::new(DIRECTORY), 0)?;
    let path = endpoint(uid);
    let listener = UnixListener::bind(&path)?;
    let metadata = fs::symlink_metadata(&path)?;
    let mut cleanup = BoundSocketCleanup {
        path: path.clone(),
        identity: (metadata.dev(), metadata.ino()),
        armed: true,
    };
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_error| denied())?;
    // SAFETY: fixed path under verified root-owned directory; no untrusted process
    // can replace it. Ownership is limited to the intended local user.
    if unsafe { libc::chown(name.as_ptr(), uid, u32::MAX) } != 0 {
        return Err(io::Error::last_os_error());
    }
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    cleanup.armed = false;
    Ok(listener)
}

// Only our just-bound socket may be removed if ownership/mode setup fails.
struct BoundSocketCleanup {
    path: PathBuf,
    identity: (u64, u64),
    armed: bool,
}
impl Drop for BoundSocketCleanup {
    fn drop(&mut self) {
        if self.armed
            && let Ok(metadata) = fs::symlink_metadata(&self.path)
            && metadata.file_type().is_socket()
            && (metadata.dev(), metadata.ino()) == self.identity
        {
            let _removed = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_bind_cleanup_preserves_a_replacement_regular_file() {
        let temp = tempfile::tempdir().expect("temporary fixture");
        let path = temp.path().join("fixture.sock");
        fs::write(&path, b"replacement").expect("replacement file");
        let metadata = fs::symlink_metadata(&path).expect("metadata");
        drop(BoundSocketCleanup {
            path: path.clone(),
            identity: (metadata.dev(), metadata.ino()),
            armed: true,
        });
        assert_eq!(fs::read(path).expect("retained replacement"), b"replacement");
    }
    #[tokio::test]
    async fn kernel_credentials_reject_a_different_user() {
        let (one, two) = UnixStream::pair().expect("pair");
        // SAFETY: geteuid only reads our identity.
        let uid = unsafe { libc::geteuid() };
        authenticate(&one, uid).expect("our user");
        authenticate(&two, uid).expect("our user");
        assert_eq!(
            authenticate(&one, uid.wrapping_add(1)).expect_err("other user").kind(),
            io::ErrorKind::PermissionDenied
        );
    }
    #[test]
    fn refuses_symlink_and_writable_runtime_directories() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let temp = tempfile::tempdir().expect("temp");
        // SAFETY: reads current uid only.
        let uid = unsafe { libc::geteuid() };
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).expect("mode");
        check_directory(temp.path(), uid).expect("private directory");
        let link = temp.path().join("link");
        symlink(temp.path(), &link).expect("link");
        assert!(check_directory(&link, uid).is_err());
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o777)).expect("mode");
        assert!(check_directory(temp.path(), uid).is_err());
    }
}

#[path = "unix_trust.rs"]
mod trust;
pub use trust::{
    MAINTENANCE_MARKER_PATH, installed_agent_path, maintenance_requested, verify_protected_path,
};

#[path = "unix_process.rs"]
mod process;
pub use process::harden_capture_process;
