//! Bounded TCP owner snapshots used only on an observed outbound SYN.
use crate::ownership::{OwnerLookup, OwnershipError};
use rokbattles_capture_ipc::windows::{Identity, ProcessIdentity};
use rokbattles_capture_runtime::packet::FlowKey;
use std::{
    io,
    mem::{offset_of, size_of},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    ptr,
};
use windows_sys::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
};
const MAX_TABLE_BYTES: usize = 4 * 1024 * 1024;
const MAX_ROWS: usize = 65_536;

pub struct WindowsOwnerLookup {
    identity: Identity,
    budget: (std::time::Instant, usize),
}
pub struct OwnedFlow {
    process: ProcessIdentity,
    pid: u32,
}
impl WindowsOwnerLookup {
    pub fn new(identity: Identity) -> Self {
        Self { identity, budget: (std::time::Instant::now(), 0) }
    }
}
impl OwnerLookup for WindowsOwnerLookup {
    type Owner = OwnedFlow;
    fn owner_for_syn(&mut self, key: FlowKey) -> Result<Option<OwnedFlow>, OwnershipError> {
        let pid = self.lookup(key, true).map_err(|_error| OwnershipError)?;
        let Some(pid) = pid else {
            return Ok(None);
        };
        let process = ProcessIdentity::open(pid).map_err(|_error| OwnershipError)?;
        if process.identity() != &self.identity || !process.is_alive() {
            return Ok(None);
        }
        // Recheck the tuple after pinning its process. The bound SYN/ISNs remain
        // mandatory; this closes lookup-to-process-open PID/tuple reuse races.
        if self.lookup(key, true).map_err(|_error| OwnershipError)? != Some(pid) {
            return Err(OwnershipError);
        }
        Ok(Some(OwnedFlow { process, pid }))
    }
    fn alive(&self, owner: &OwnedFlow) -> bool {
        owner.process.is_alive()
    }
    fn still_owner(&mut self, key: FlowKey, owner: &OwnedFlow) -> bool {
        if !owner.process.is_alive() || self.lookup(key, false).ok().flatten() != Some(owner.pid) {
            return false;
        }
        let Ok(current) = ProcessIdentity::open(owner.pid) else {
            return false;
        };
        current.creation_time() == owner.process.creation_time()
            && current.identity() == &self.identity
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "ambiguous TCP owner snapshot")
}

fn unique_owner(key: FlowKey, initial: bool) -> io::Result<Option<u32>> {
    let family = if key.client.is_ipv4() { AF_INET } else { AF_INET6 };
    let (buffer, returned) = snapshot(u32::from(family))?;
    let bytes = as_bytes(&buffer).get(..returned).ok_or_else(invalid)?;
    let count = u32::from_ne_bytes(
        bytes.get(..4).ok_or_else(invalid)?.try_into().map_err(|_error| invalid())?,
    ) as usize;
    if count > MAX_ROWS {
        return Err(invalid());
    }
    let (offset, stride) = if family == AF_INET {
        (offset_of!(MIB_TCPTABLE_OWNER_PID, table), size_of::<MIB_TCPROW_OWNER_PID>())
    } else {
        (offset_of!(MIB_TCP6TABLE_OWNER_PID, table), size_of::<MIB_TCP6ROW_OWNER_PID>())
    };
    let rows = bytes
        .get(
            offset
                ..offset
                    .checked_add(count.checked_mul(stride).ok_or_else(invalid)?)
                    .ok_or_else(invalid)?,
        )
        .ok_or_else(invalid)?;
    let mut owner = None;
    for row in rows.chunks_exact(stride) {
        let (tuple, pid, state) = if family == AF_INET {
            // SAFETY: exact row extent checked by chunks_exact; unaligned copy has no pointers.
            let row = unsafe { ptr::read_unaligned(row.as_ptr().cast::<MIB_TCPROW_OWNER_PID>()) };
            (
                FlowKey {
                    client: SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes())),
                        port(row.dwLocalPort)?,
                    ),
                    server: SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::from(row.dwRemoteAddr.to_ne_bytes())),
                        port(row.dwRemotePort)?,
                    ),
                },
                row.dwOwningPid,
                row.dwState,
            )
        } else {
            // SAFETY: exact row extent checked by chunks_exact; unaligned copy has no pointers.
            let row = unsafe { ptr::read_unaligned(row.as_ptr().cast::<MIB_TCP6ROW_OWNER_PID>()) };
            if row.dwLocalScopeId != 0 || row.dwRemoteScopeId != 0 {
                continue;
            }
            (
                FlowKey {
                    client: SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)),
                        port(row.dwLocalPort)?,
                    ),
                    server: SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::from(row.ucRemoteAddr)),
                        port(row.dwRemotePort)?,
                    ),
                },
                row.dwOwningPid,
                row.dwState,
            )
        };
        if tuple != key {
            continue;
        }
        // SYN_SENT/SYN_RCVD/ESTABLISHED only. Listeners, closed and TIME_WAIT
        // entries cannot establish ownership of a captured outbound SYN.
        if !admissible_state(state, initial) || pid == 0 || owner.is_some() {
            return Err(invalid());
        }
        owner = Some(pid);
    }
    Ok(owner)
}
fn port(raw: u32) -> io::Result<u16> {
    if raw > u16::MAX as u32 {
        return Err(invalid());
    }
    Ok(u16::from_be(raw as u16))
}
fn snapshot(family: u32) -> io::Result<(Vec<u64>, usize)> {
    let mut size = 0;
    // SAFETY: documented size-only query; no output buffer is supplied.
    let result = unsafe {
        GetExtendedTcpTable(ptr::null_mut(), &mut size, 0, family, TCP_TABLE_OWNER_PID_ALL, 0)
    };
    if result != ERROR_INSUFFICIENT_BUFFER || size as usize > MAX_TABLE_BYTES || size < 4 {
        return Err(invalid());
    }
    let mut buffer = vec![0u64; (size as usize).div_ceil(size_of::<u64>())];
    // SAFETY: aligned initialized output with at least the returned allocation size.
    let result = unsafe {
        GetExtendedTcpTable(
            buffer.as_mut_ptr().cast(),
            &mut size,
            0,
            family,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if result != ERROR_SUCCESS || size as usize > buffer.len() * size_of::<u64>() {
        return Err(invalid());
    }
    // No retry loop on racing/oversized tables: fail closed with a local Gap.
    Ok((buffer, size as usize))
}
fn as_bytes(buffer: &[u64]) -> &[u8] {
    // SAFETY: u64 has no padding, all bytes initialized, slice lifetime bound to buffer.
    unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast(), std::mem::size_of_val(buffer)) }
}

impl WindowsOwnerLookup {
    fn lookup(&mut self, key: FlowKey, initial: bool) -> io::Result<Option<u32>> {
        if self.budget.0.elapsed() >= std::time::Duration::from_secs(1) {
            self.budget = (std::time::Instant::now(), 0);
        }
        if self.budget.1 >= 256 {
            return Err(invalid());
        }
        self.budget.1 += 1;
        unique_owner(key, initial)
    }
}

fn admissible_state(state: u32, initial: bool) -> bool {
    // Established and live closing states retain exact ownership. In particular,
    // FIN_WAIT1/2 must keep receiving the server half of a client half-close.
    if initial { matches!(state, 3..=5) } else { matches!(state, 3..=10) }
}
#[cfg(test)]
mod tests {
    #[test]
    fn owner_revalidation_preserves_live_half_close_states() {
        for state in [6, 7, 8, 9, 10] {
            assert!(super::admissible_state(state, false));
            assert!(!super::admissible_state(state, true));
        }
        for state in [1, 2, 11, 12] {
            assert!(!super::admissible_state(state, false));
        }
    }
}
