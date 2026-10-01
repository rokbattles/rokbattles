//! macOS libproc ownership: UID + process birth + FD + socket generation, with
//! SDK-native C accessors. Every final writer lookup obtains fresh kernel data.
use crate::ownership::{OwnerLookup, OwnershipError};
use rokbattles_capture_runtime::packet::FlowKey;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

const MAX_PROCESSES: usize = 16_384;
const MAX_SOCKETS: usize = 16_384;
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct NativeProcess {
    start_sec: u64,
    start_usec: u64,
    pid: u32,
    uid: u32,
    ruid: u32,
    svuid: u32,
    status: u32,
    flags: u32,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct NativeSocket {
    generation: u64,
    socket_id: u64,
    local: [u8; 16],
    remote: [u8; 16],
    uid: u32,
    state: u32,
    shared: u32,
    fd: i32,
    local_port: u16,
    remote_port: u16,
    version: u8,
    reserved: [u8; 3],
}
unsafe extern "C" {
    fn rb_process_read(pid: i32, out: *mut NativeProcess) -> i32;
    fn rb_user_pids(uid: u32, out: *mut i32, capacity: u32) -> i32;
    fn rb_sockets(pid: i32, out: *mut NativeSocket, capacity: u32) -> i32;
}
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "socket ownership unavailable")
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    uid: u32,
    start_sec: u64,
    start_usec: u64,
}
impl ProcessIdentity {
    pub fn read(pid: u32, uid: u32) -> io::Result<Self> {
        let pid_native = i32::try_from(pid).map_err(|_error| denied())?;
        if pid_native <= 0 || uid == 0 {
            return Err(denied());
        }
        let mut native = NativeProcess::default();
        // SAFETY: bridge has exact scalar ABI and initializes the entire output.
        if unsafe { rb_process_read(pid_native, &mut native) } != 0 {
            return Err(denied());
        }
        if !valid_process(native, pid, uid) {
            return Err(denied());
        }
        Ok(Self { pid, uid, start_sec: native.start_sec, start_usec: native.start_usec })
    }
    pub fn is_alive(&self) -> bool {
        Self::read(self.pid, self.uid).is_ok_and(|current| current == *self)
    }
}
fn valid_process(native: NativeProcess, pid: u32, uid: u32) -> bool {
    native.pid == pid
        && native.uid == uid
        && native.ruid == uid
        && native.svuid == uid
        && native.status != 5
        && native.flags & 4 == 0
        && native.start_sec != 0
        && native.start_usec < 1_000_000
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SocketOwner {
    process: ProcessIdentity,
    fd: i32,
    generation: u64,
    socket_id: u64,
}
pub struct UnixOwnerLookup {
    uid: u32,
}
impl UnixOwnerLookup {
    pub fn new(uid: u32) -> Self {
        Self { uid }
    }
}
fn pids(uid: u32) -> io::Result<Vec<i32>> {
    let mut rows = vec![0i32; MAX_PROCESSES];
    // SAFETY: contiguous initialized output is exactly capacity i32 values.
    let count = unsafe { rb_user_pids(uid, rows.as_mut_ptr(), MAX_PROCESSES as u32) };
    let count = usize::try_from(count).map_err(|_error| denied())?;
    if count >= rows.len() {
        return Err(denied());
    }
    rows.truncate(count);
    rows.retain(|pid| *pid > 0);
    Ok(rows)
}
fn sockets(pid: u32) -> io::Result<Vec<NativeSocket>> {
    let mut rows = vec![NativeSocket::default(); MAX_SOCKETS];
    let pid = i32::try_from(pid).map_err(|_error| denied())?;
    // SAFETY: C bridge caps writes at the supplied capacity and rejects truncation.
    let count = unsafe { rb_sockets(pid, rows.as_mut_ptr(), MAX_SOCKETS as u32) };
    let count = usize::try_from(count).map_err(|_error| denied())?;
    if count > rows.len() {
        return Err(denied());
    }
    rows.truncate(count);
    Ok(rows)
}
fn key(row: NativeSocket) -> io::Result<FlowKey> {
    let address = |bytes: [u8; 16], port| -> io::Result<SocketAddr> {
        let ip = match row.version {
            4 if bytes.get(..12) == Some(&[0; 12]) => IpAddr::V4(Ipv4Addr::from(
                <[u8; 4]>::try_from(bytes.get(12..).ok_or_else(denied)?)
                    .map_err(|_error| denied())?,
            )),
            6 => IpAddr::V6(Ipv6Addr::from(bytes)),
            _ => return Err(denied()),
        };
        Ok(SocketAddr::new(ip, port))
    };
    Ok(FlowKey {
        client: address(row.local, row.local_port)?,
        server: address(row.remote, row.remote_port)?,
    })
}
fn live(row: NativeSocket, uid: u32) -> bool {
    row.uid == uid
        && row.shared == 0
        && row.generation != 0
        && row.socket_id != 0
        && row.fd >= 0
        && matches!(row.state, 2..=9)
}
impl OwnerLookup for UnixOwnerLookup {
    type Owner = SocketOwner;
    fn owner_for_syn(&mut self, flow: FlowKey) -> Result<Option<SocketOwner>, OwnershipError> {
        self.find_owner(flow).map_err(|_error| OwnershipError)
    }
    fn alive(&self, owner: &SocketOwner) -> bool {
        owner.process.is_alive()
    }
    fn still_owner(&mut self, flow: FlowKey, owner: &SocketOwner) -> bool {
        if !owner.process.is_alive() {
            return false;
        }
        let rows = match sockets(owner.process.pid) {
            Ok(rows) => rows,
            Err(_) => return false,
        };
        let mut matches = rows.into_iter().filter(|row| key(*row).ok() == Some(flow));
        let Some(row) = matches.next() else {
            return false;
        };
        matches.next().is_none()
            && live(row, self.uid)
            && row.fd == owner.fd
            && row.generation == owner.generation
            && row.socket_id == owner.socket_id
            && owner.process.is_alive()
    }
}
impl UnixOwnerLookup {
    fn find_owner(&self, flow: FlowKey) -> io::Result<Option<SocketOwner>> {
        let mut matched = None;
        for pid in pids(self.uid)? {
            let pid = u32::try_from(pid).map_err(|_error| denied())?;
            let process = ProcessIdentity::read(pid, self.uid)?;
            for row in sockets(pid)? {
                if key(row)? != flow {
                    continue;
                }
                if !live(row, self.uid) || !matches!(row.state, 2 | 4) || !process.is_alive() {
                    return Err(denied());
                }
                let owner = SocketOwner {
                    process,
                    fd: row.fd,
                    generation: row.generation,
                    socket_id: row.socket_id,
                };
                if matched.replace(owner).is_some() {
                    return Err(denied());
                }
            }
        }
        Ok(matched)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_layout_is_independent_of_sdk_socket_layout() {
        assert_eq!(std::mem::size_of::<NativeProcess>(), 40);
        assert_eq!(std::mem::size_of::<NativeSocket>(), 72);
        assert_eq!(std::mem::offset_of!(NativeSocket, fd), 60);
    }
    #[test]
    fn shared_missing_and_foreign_socket_identity_fails_closed() {
        let mut row = NativeSocket {
            uid: 501,
            generation: 11,
            socket_id: 12,
            fd: 3,
            state: 6,
            ..Default::default()
        };
        assert!(live(row, 501)); // FIN_WAIT_1 keeps the server tail.
        row.shared = 1;
        assert!(!live(row, 501));
        row.shared = 0;
        assert!(!live(row, 502));
        row.generation = 0;
        assert!(!live(row, 501));
    }
    #[test]
    fn process_uid_birth_and_exit_flags_are_required() {
        let mut native = NativeProcess {
            pid: 8,
            uid: 501,
            ruid: 501,
            svuid: 501,
            start_sec: 1,
            status: 2,
            ..Default::default()
        };
        assert!(valid_process(native, 8, 501));
        native.ruid = 502;
        assert!(!valid_process(native, 8, 501));
        native.ruid = 501;
        native.flags = 4;
        assert!(!valid_process(native, 8, 501));
    }
}
