use super::{ProcessIdentity, UnixOwnerLookup, agent::AgentPeer, pump::Pump};
use crate::ownership::FlowGuard;
use rokbattles_capture_ipc::{
    ClientRequest, IO_DEADLINE, Record, read_request,
    unix::{authenticate, bind_for_user},
    write_record,
};
use std::{
    fs, io,
    os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::{net::UnixStream, sync::watch};

const DIRECTORY: &str = "/var/run/rokbattles-capture";
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "capture service identity rejected")
}

/// Only the OS service's fixed executable and numeric per-user instance may run.
/// Provisioning is a separate explicit administrator action, never performed here.
pub fn dispatch(uid: u32) -> io::Result<()> {
    super::trust::verify_service()?;
    if uid == 0 || uid == u32::MAX {
        return Err(denied());
    }
    // SAFETY: called before constructing any runtime/thread, process-local umask.
    unsafe { libc::umask(0o077) };
    super::bootstrap::prepare()?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(run(uid))
}
async fn run(uid: u32) -> io::Result<()> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let lease = SocketLease::acquire(uid)?;
    let listener = bind_for_user(uid)?;
    let lease = lease.bound()?;
    let (cancel, receiver) = watch::channel(false);
    let mut task = tokio::spawn(serve(listener, uid, receiver));
    tokio::select! {
        _ = terminate.recv() => {},
        _ = interrupt.recv() => {},
        result = &mut task => { drop(lease); return result.map_err(io::Error::other)?; },
    }
    let _sent = cancel.send(true);
    if !matches!(tokio::time::timeout(Duration::from_secs(8), task).await, Ok(Ok(_))) {
        std::process::abort();
    }
    drop(lease);
    Ok(())
}
async fn serve(
    listener: tokio::net::UnixListener,
    uid: u32,
    mut cancel: watch::Receiver<bool>,
) -> io::Result<()> {
    while !*cancel.borrow() {
        let (mut stream, _) = tokio::select! {
            biased;
            _ = cancel.changed() => break,
            connection = listener.accept() => connection?,
        };
        if authenticate(&stream, uid).is_err() {
            continue;
        }
        let peer = match peer(&stream, uid) {
            Ok(peer) => peer,
            Err(_) => continue,
        };
        let started = tokio::select! {
            biased;
            _ = cancel.changed() => break,
            request = tokio::time::timeout(IO_DEADLINE, read_request(&mut stream)) => request,
        };
        if matches!(started, Ok(Ok(ClientRequest::Start))) && peer.is_alive() {
            session(&mut stream, uid, peer, &mut cancel).await;
        }
        // Closing the authenticated connection ends its consent and all ownership.
    }
    Ok(())
}
async fn session(
    stream: &mut UnixStream,
    uid: u32,
    peer: AgentPeer,
    cancel: &mut watch::Receiver<bool>,
) {
    let mut pump = Pump::start();
    let mut guard = FlowGuard::new(UnixOwnerLookup::new(uid));
    let started = Instant::now();
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut request = Box::pin(read_request(&mut reader));
    let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
    let mut stopped = false;
    let mut has_started = false;
    loop {
        tokio::select! {
            biased;
            _ = cancel.changed() => break,
            _ = pump.failed.changed() => {
                let end = if has_started { guard.gap() } else { Record::Unavailable(rokbattles_capture_ipc::UnavailableReason::NativeBackend) };
                let _written = write_record(&mut writer, &end).await;
                break;
            },
            result = &mut request => {
                pump.cancel();
                stopped = matches!(result, Ok(ClientRequest::Stop));
                break;
            },
            next = pump.records.recv() => {
                let Some(record) = next else { break; };
                if !peer.is_alive() || *pump.failed.borrow() || *cancel.borrow() { break; }
                let unavailable = matches!(record, Record::Unavailable(_));
                has_started |= matches!(record, Record::Started(_));
                let records = match record {
                    Record::ServerPacket(bytes) => guard.server(bytes, started.elapsed()),
                    Record::ClientControl(control) => guard.client(control, started.elapsed()),
                    other => vec![other],
                };
                let mut failed = false;
                for record in records {
                    // An awaited previous write may outlive consent, process or
                    // socket ownership. Never reuse authorization from capture time.
                    if *cancel.borrow() || *pump.failed.borrow() || !peer.is_alive() { failed = true; break; }
                    if !guard.authorize_record(&record) {
                        if write_record(&mut writer, &guard.gap()).await.is_err() { failed = true; }
                        break;
                    }
                    if write_record(&mut writer, &record).await.is_err() { failed = true; break; }
                    guard.record_written(&record);
                }
                if failed || unavailable { break; }
            },
            _ = heartbeat.tick() => {
                if !peer.is_alive() || *cancel.borrow() { break; }
                if !has_started { continue; }
                let mut failed = false;
                for record in guard.expire(started.elapsed()).into_iter().chain([Record::Keepalive]) {
                    if write_record(&mut writer, &record).await.is_err() { failed = true; break; }
                }
                if failed { break; }
            },
        }
    }
    pump.cancel();
    pump.finish().await;
    if stopped {
        let _written = write_record(&mut writer, &Record::Stopped).await;
    }
}

fn peer(stream: &UnixStream, uid: u32) -> io::Result<AgentPeer> {
    authenticate(stream, uid)?;
    #[cfg(target_os = "linux")]
    let pid = {
        let mut credentials = libc::ucred { pid: 0, uid: 0, gid: 0 };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: valid socket, exact kernel struct and writable size/output.
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
        if length as usize != std::mem::size_of::<libc::ucred>() || credentials.uid != uid {
            return Err(denied());
        }
        u32::try_from(credentials.pid).map_err(|_error| denied())?
    };
    #[cfg(target_os = "macos")]
    let pid = {
        let mut pid: libc::pid_t = 0;
        let mut length = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
        // SAFETY: LOCAL_PEERPID returns the kernel peer PID for this Unix socket.
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                0,
                libc::LOCAL_PEERPID,
                (&mut pid as *mut libc::pid_t).cast(),
                &mut length,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if length as usize != std::mem::size_of::<libc::pid_t>() {
            return Err(denied());
        }
        u32::try_from(pid).map_err(|_error| denied())?
    };
    AgentPeer::open(ProcessIdentity::read(pid, uid)?)
}

/// Exclusive root-owned per-UID lock permits safe stale-socket recovery. A user
/// cannot create either file in the verified directory. Never unlink other types.
struct SocketLease {
    _lock: fs::File,
    path: PathBuf,
    inode: Option<(u64, u64)>,
}
impl SocketLease {
    fn acquire(uid: u32) -> io::Result<Self> {
        let directory = fs::symlink_metadata(DIRECTORY)?;
        if !directory.is_dir() || directory.uid() != 0 || directory.mode() & 0o022 != 0 {
            return Err(denied());
        }
        let root = Path::new(DIRECTORY);
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root.join(format!("{uid}.lock")))?;
        let metadata = lock.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != 0
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err(denied());
        }
        // SAFETY: flock only operates on this owned file descriptor.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let path = root.join(format!("{uid}.sock"));
        match fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_socket()
                    && metadata.uid() == uid
                    && metadata.mode() & 0o077 == 0 =>
            {
                fs::remove_file(&path)?
            }
            Ok(_) => return Err(denied()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(Self { _lock: lock, path, inode: None })
    }
    fn bound(mut self) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(&self.path)?;
        self.inode = Some((metadata.dev(), metadata.ino()));
        Ok(self)
    }
}
impl Drop for SocketLease {
    fn drop(&mut self) {
        if let Some(expected) = self.inode
            && let Ok(metadata) = fs::symlink_metadata(&self.path)
            && metadata.file_type().is_socket()
            && (metadata.dev(), metadata.ino()) == expected
        {
            let _removed = fs::remove_file(&self.path);
        }
    }
}
