//! Only the fixed protected installed agent can receive privileged records.
use super::ProcessIdentity;
use std::{
    fs, io,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

#[cfg(target_os = "linux")]
pub const AGENT: &str = "/usr/libexec/rokbattles/rokbattles-desktop-agent";
#[cfg(target_os = "macos")]
pub const AGENT: &str = "/Library/Application Support/ROK Battles/rokbattles-desktop-agent";
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "protected installed agent required")
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl FileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> io::Result<Self> {
        if !metadata.is_file()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || metadata.mode() & 0o111 == 0
        {
            return Err(denied());
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

pub struct AgentPeer {
    process: ProcessIdentity,
    image: fs::File,
    identity: FileIdentity,
}
impl AgentPeer {
    pub fn open(process: ProcessIdentity) -> io::Result<Self> {
        let canonical = rokbattles_capture_ipc::unix::installed_agent_path()?;
        // No alias installs: launch and authentication use this same literal path.
        if canonical != Path::new(AGENT) {
            return Err(denied());
        }
        let image = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical)?;
        let identity = FileIdentity::from_metadata(&image.metadata()?)?;
        let peer = Self { process, image, identity };
        if !peer.is_alive() {
            return Err(denied());
        }
        Ok(peer)
    }
    pub fn is_alive(&self) -> bool {
        self.process.is_alive() && self.current_image().is_ok() && self.process.is_alive()
    }
    fn current_image(&self) -> io::Result<()> {
        rokbattles_capture_ipc::unix::verify_protected_path(Path::new(AGENT))?;
        if FileIdentity::from_metadata(&self.image.metadata()?)? != self.identity
            || FileIdentity::from_metadata(&fs::symlink_metadata(AGENT)?)? != self.identity
        {
            return Err(denied());
        }
        #[cfg(target_os = "linux")]
        {
            let executable = format!("/proc/{}/exe", self.process.pid);
            let running = fs::File::open(&executable)?;
            if fs::read_link(&executable)? != Path::new(AGENT)
                || FileIdentity::from_metadata(&running.metadata()?)? != self.identity
            {
                return Err(denied());
            }
            use std::io::Read;
            let mut status = String::new();
            fs::File::open(format!("/proc/{}/status", self.process.pid))?
                .take(65537)
                .read_to_string(&mut status)?;
            if status.len() > 65536 || !untraced(&status) {
                return Err(denied());
            }
        }
        #[cfg(target_os = "macos")]
        {
            unsafe extern "C" {
                fn rb_agent_valid(pid: i32) -> i32;
            }
            let pid = i32::try_from(self.process.pid).map_err(|_error| denied())?;
            // SAFETY: read-only Security/libproc bridge takes only a positive PID;
            // the process birth and fixed root-owned image are checked on both sides.
            if unsafe { rb_agent_valid(pid) } != 0 {
                return Err(denied());
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn untraced(status: &str) -> bool {
    let mut values = status.lines().filter_map(|line| line.strip_prefix("TracerPid:"));
    values.next().is_some_and(|value| value.trim() == "0") && values.next().is_none()
}
#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn tracer_identity_must_be_present_unique_and_zero() {
        assert!(super::untraced("Name: agent\nTracerPid:\t0\n"));
        for text in ["", "TracerPid: 1", "TracerPid: 0\nTracerPid: 1", "TracerPid: -0"] {
            assert!(!super::untraced(text));
        }
    }
}
