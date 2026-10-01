//! Fail-closed Windows socket evidence for a server-only Npcap capture.
//!
//! A table row is not a packet observation. Admission requires a new, pinned
//! OWNER_MODULE row observed in SYN_SENT and then ESTABLISHED, and a captured
//! server SYN/ACK. Its ACK establishes the client ISN without inventing a client
//! packet. Every forwarded packet gets a new table and process-identity check.
//! Existing, missed-handshake, retired, or conflicting tuples stay excluded for
//! this capture. Independent tuples have independent provisional lifetimes.

use rokbattles_capture_runtime::{
    SERVER_PORTS,
    packet::{FlowKey, ServerPacket, SocketEstablishedEvidence, SocketRetiredEvidence},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

const MAX_ROWS: usize = 16_384;
const MAX_TABLE_BYTES: usize = 4 * 1024 * 1024;
const MAX_FLOWS: usize = 32;
const MAX_EXCLUDED: usize = 32_768;
const MAX_POLLS_PER_SECOND: u32 = 128;
const MAX_PACKETS_PER_SECOND: u32 = 512;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const FLOW_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SEQUENCE_WINDOW: u32 = 4 * 1024 * 1024;
/// Keep one allocator in the authenticated IPC session for one Observer lifetime.
/// Reuse it across source recreation after a gap; never share it across logons.
/// A replacement transport/process must create a fresh Observer and allocator.
/// Requests cannot provide, reset, or choose its sequence.
#[derive(Clone)]
pub struct SessionGenerations(Arc<AtomicU64>);
impl Default for SessionGenerations {
    fn default() -> Self {
        Self(Arc::new(AtomicU64::new(1)))
    }
}
impl SessionGenerations {
    fn next(&self) -> Result<u64, EvidenceError> {
        let mut current = self.0.load(Ordering::Relaxed);
        loop {
            let next = current.checked_add(1).ok_or(EvidenceError)?;
            match self.0.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(value) => return Ok(value),
                Err(observed) => current = observed,
            }
        }
    }
}
/// The host polls independently of packet arrival so a short SYN_SENT can be witnessed.
/// Missing that transition intentionally makes the flow ineligible.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// No process identifiers, addresses, token details, or bytes appear in errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceError;
impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("socket ownership evidence unavailable")
    }
}
impl std::error::Error for EvidenceError {}

#[derive(Debug)]
pub enum EvidenceEvent {
    Established(SocketEstablishedEvidence),
    Retired(SocketRetiredEvidence),
}
/// Emit all events, in order, before forwarding the current server packet.
#[derive(Debug)]
pub struct ServerDecision {
    pub events: Vec<EvidenceEvent>,
    pub allowed: bool,
}

// Values are MIB_TCP_STATE, documented by Microsoft. Keep OS decoding separate
// from the portable model so synthetic snapshots exercise the real policy.
const CLOSED: u32 = 1;
const SYN_SENT: u32 = 3;
const ESTABLISHED: u32 = 5;
const FIN_WAIT1: u32 = 6;
const FIN_WAIT2: u32 = 7;
const CLOSE_WAIT: u32 = 8;
const CLOSING: u32 = 9;
const LAST_ACK: u32 = 10;
const TIME_WAIT: u32 = 11;
const DELETE_TCB: u32 = 12;

#[derive(Clone, Copy)]
struct Row {
    key: FlowKey,
    pid: u32,
    created: u64,
    state: u32,
}
impl Row {
    fn eligible_key(self) -> bool {
        fn unicast(ip: std::net::IpAddr) -> bool {
            match ip {
                std::net::IpAddr::V4(ip) => {
                    !ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast()
                }
                std::net::IpAddr::V6(ip) => !ip.is_unspecified() && !ip.is_multicast(),
            }
        }
        self.key.client.is_ipv4() == self.key.server.is_ipv4()
            && SERVER_PORTS.contains(&self.key.server.port())
            && self.key.client.port() != 0
            && !SERVER_PORTS.contains(&self.key.client.port())
            && unicast(self.key.client.ip())
            && unicast(self.key.server.ip())
    }
    fn same_instance(self, other: Self) -> bool {
        self.key == other.key && self.pid == other.pid && self.created == other.created
    }
}

trait Owners {
    type Pin;
    /// Pin an OS process only if its live token belongs to the authenticated user/logon.
    fn pin(&mut self, row: Row) -> Result<Option<Self::Pin>, EvidenceError>;
    /// A no-output poll checks handle liveness without reopening every token.
    fn alive(&self, pin: &Self::Pin) -> bool;
    /// Recheck the pinned process AND its current token before any later admission.
    fn matches(&mut self, pin: &Self::Pin, row: Row) -> Result<bool, EvidenceError>;
}

#[derive(Clone, Copy)]
struct Handshake {
    client_isn: u32,
    server_isn: u32,
}
struct Flow<P> {
    row: Row,
    pin: P,
    generation: u64,
    handshake: Option<Handshake>,
    established: bool,
    announced: bool,
    server_next: u32,
    client_ack: u32,
    touched: Duration,
}
impl<P> Flow<P> {
    fn established_event(
        &mut self,
        generations: &SessionGenerations,
    ) -> Result<Option<EvidenceEvent>, EvidenceError> {
        if !self.established || self.announced {
            return Ok(None);
        }
        let Some(handshake) = self.handshake else {
            return Ok(None);
        };
        // Allocate only when emitting: concurrent handshakes can finish out of order.
        self.generation = generations.next()?;
        self.announced = true;
        Ok(Some(EvidenceEvent::Established(SocketEstablishedEvidence {
            key: self.row.key,
            client_initial_sequence: handshake.client_isn,
            server_initial_sequence: handshake.server_isn,
            generation: self.generation,
        })))
    }
    fn retirement(&self) -> Option<EvidenceEvent> {
        if !self.announced {
            return None;
        }
        Some(EvidenceEvent::Retired(SocketRetiredEvidence {
            key: self.row.key,
            client_initial_sequence: self.handshake?.client_isn,
            generation: self.generation,
        }))
    }
}

struct EvidenceState<P> {
    cutoff: u64,
    flows: BTreeMap<FlowKey, Flow<P>>,
    excluded: BTreeSet<FlowKey>,
    last_now: Duration,
    failed: bool,
    generations: SessionGenerations,
}
impl<P> EvidenceState<P> {
    fn baseline(
        rows: Vec<Row>,
        cutoff: u64,
        generations: SessionGenerations,
    ) -> Result<Self, EvidenceError> {
        if rows.len() > MAX_ROWS || cutoff == 0 {
            return Err(EvidenceError);
        }
        // All existing rows are excluded, irrespective of current state or owner.
        let excluded = rows.into_iter().map(|row| row.key).collect();
        Ok(Self {
            cutoff,
            flows: BTreeMap::new(),
            excluded,
            last_now: Duration::ZERO,
            failed: false,
            generations,
        })
    }

    fn poison(&mut self) {
        self.flows.clear();
        self.failed = true;
    }

    fn exclude(
        &mut self,
        key: FlowKey,
        events: &mut Vec<EvidenceEvent>,
    ) -> Result<(), EvidenceError> {
        if let Some(flow) = self.flows.remove(&key)
            && let Some(event) = flow.retirement()
        {
            events.push(event);
        }
        if !self.excluded.contains(&key) && self.excluded.len() >= MAX_EXCLUDED {
            return Err(EvidenceError);
        }
        self.excluded.insert(key);
        Ok(())
    }

    fn observe<O: Owners<Pin = P>>(
        &mut self,
        rows: Vec<Row>,
        wall_time: u64,
        now: Duration,
        owners: &mut O,
    ) -> Result<Vec<EvidenceEvent>, EvidenceError> {
        self.observe_for(rows, wall_time, now, owners, None)
    }

    fn observe_for<O: Owners<Pin = P>>(
        &mut self,
        rows: Vec<Row>,
        wall_time: u64,
        now: Duration,
        owners: &mut O,
        release: Option<FlowKey>,
    ) -> Result<Vec<EvidenceEvent>, EvidenceError> {
        let result = self.observe_inner(rows, wall_time, now, owners, release);
        if result.is_err() {
            self.poison();
        }
        result
    }

    fn observe_inner<O: Owners<Pin = P>>(
        &mut self,
        rows: Vec<Row>,
        wall_time: u64,
        now: Duration,
        owners: &mut O,
        release: Option<FlowKey>,
    ) -> Result<Vec<EvidenceEvent>, EvidenceError> {
        if self.failed || now < self.last_now || wall_time < self.cutoff || rows.len() > MAX_ROWS {
            return Err(EvidenceError);
        }
        self.last_now = now;
        let mut current = BTreeMap::new();
        for row in rows.into_iter().filter(|row| row.eligible_key()) {
            // Multiple OS rows for one representable tuple cannot identify a flow.
            if current.insert(row.key, row).is_some() {
                return Err(EvidenceError);
            }
        }
        let mut events = Vec::new();
        let mut retired = Vec::new();
        for (key, flow) in &mut self.flows {
            let Some(row) = current.get(key).copied() else {
                retired.push(*key);
                continue;
            };
            let timeout = if flow.announced { FLOW_IDLE_TIMEOUT } else { HANDSHAKE_TIMEOUT };
            if !flow.row.same_instance(row)
                || now.saturating_sub(flow.touched) > timeout
                || !owners.alive(&flow.pin)
            {
                retired.push(*key);
                continue;
            }
            let progressed = match flow.row.state {
                SYN_SENT => matches!(row.state, SYN_SENT | ESTABLISHED),
                ESTABLISHED => {
                    matches!(row.state, ESTABLISHED | FIN_WAIT1 | FIN_WAIT2 | CLOSE_WAIT)
                }
                FIN_WAIT1 => matches!(row.state, FIN_WAIT1 | FIN_WAIT2 | CLOSING),
                FIN_WAIT2 => matches!(row.state, FIN_WAIT2),
                CLOSE_WAIT => matches!(row.state, CLOSE_WAIT | LAST_ACK),
                CLOSING => matches!(row.state, CLOSING),
                LAST_ACK => matches!(row.state, LAST_ACK),
                _ => false,
            };
            if matches!(row.state, CLOSED | TIME_WAIT | DELETE_TCB) || !progressed {
                retired.push(*key);
                continue;
            }
            flow.row = row;
            if row.state == ESTABLISHED {
                flow.established = true;
            }
            let will_announce = flow.established && flow.handshake.is_some() && !flow.announced;
            if (will_announce || release == Some(*key)) && !owners.matches(&flow.pin, row)? {
                retired.push(*key);
                continue;
            }
            if let Some(event) = flow.established_event(&self.generations)? {
                events.push(event);
            }
        }
        for key in retired {
            self.exclude(key, &mut events)?;
        }
        for (key, row) in current {
            if self.flows.contains_key(&key) || self.excluded.contains(&key) {
                continue;
            }
            // Creation time supplements the baseline: rows bound before start,
            // including sockets absent during the first table read, stay excluded.
            if row.state != SYN_SENT
                || row.created <= self.cutoff
                || row.created > wall_time
                || row.pid == 0
            {
                self.exclude(key, &mut events)?;
                continue;
            }
            if self.flows.len() >= MAX_FLOWS {
                return Err(EvidenceError);
            }
            let Some(pin) = owners.pin(row)? else {
                self.exclude(key, &mut events)?;
                continue;
            };
            self.flows.insert(
                key,
                Flow {
                    row,
                    pin,
                    generation: 0,
                    handshake: None,
                    established: false,
                    announced: false,
                    server_next: 0,
                    client_ack: 0,
                    touched: now,
                },
            );
        }
        Ok(events)
    }

    /// Called only immediately after observe using the fresh snapshot for this packet.
    fn server(
        &mut self,
        packet: &ServerPacket<'_>,
        now: Duration,
        mut events: Vec<EvidenceEvent>,
    ) -> Result<ServerDecision, EvidenceError> {
        let result = self.server_inner(packet, now, &mut events);
        match result {
            Ok(allowed) => Ok(ServerDecision { events, allowed }),
            Err(error) => {
                self.poison();
                Err(error)
            }
        }
    }

    fn server_inner(
        &mut self,
        packet: &ServerPacket<'_>,
        now: Duration,
        events: &mut Vec<EvidenceEvent>,
    ) -> Result<bool, EvidenceError> {
        if self.failed || now != self.last_now {
            return Err(EvidenceError);
        }
        let key = packet.flow_key();
        let Some(flow) = self.flows.get_mut(&key) else {
            return Ok(false);
        };
        let flags = packet.flags() & 0x3f;
        if flags & 0x02 != 0 {
            let handshake = Handshake {
                client_isn: packet.acknowledgement().wrapping_sub(1),
                server_isn: packet.sequence(),
            };
            if flags != 0x12
                || packet.payload_len() != 0
                || flow.handshake.is_some_and(|old| {
                    old.client_isn != handshake.client_isn || old.server_isn != handshake.server_isn
                })
            {
                self.exclude(key, events)?;
                return Ok(false);
            }
            if flow.handshake.is_none() {
                flow.handshake = Some(handshake);
                flow.server_next = handshake.server_isn.wrapping_add(1);
                flow.client_ack = handshake.client_isn.wrapping_add(1);
            }
            if let Some(event) = flow.established_event(&self.generations)? {
                events.push(event);
            }
            // Evidence includes both ISNs. Do not forward a redundant SYN/ACK to
            // the runtime after it has initialized the attested generation.
            return Ok(false);
        }
        if !flow.announced {
            // A packet before complete evidence would lose the first server bytes.
            // Permanently exclude the tuple instead of later adopting its stream.
            self.exclude(key, events)?;
            return Ok(false);
        }
        let acknowledged = flags & 0x10 != 0;
        if !near(packet.sequence(), flow.server_next)
            || (acknowledged && !near(packet.acknowledgement(), flow.client_ack))
            || (!acknowledged && flags & 0x04 == 0)
        {
            self.exclude(key, events)?;
            return Ok(false);
        }
        if flags & 0x04 != 0 {
            // A reset ends the attested generation immediately; never let later
            // queued bytes inherit its authority, even if the OS row lingers.
            self.exclude(key, events)?;
            return Ok(false);
        }
        let length = u32::try_from(packet.payload_len()).map_err(|_error| EvidenceError)?;
        advance(
            &mut flow.server_next,
            packet.sequence().wrapping_add(length).wrapping_add(u32::from(flags & 0x01 != 0)),
        );
        if acknowledged {
            advance(&mut flow.client_ack, packet.acknowledgement());
        }
        flow.touched = now;
        Ok(true)
    }
}
fn near(value: u32, reference: u32) -> bool {
    value.wrapping_sub(reference) <= SEQUENCE_WINDOW
        || reference.wrapping_sub(value) <= SEQUENCE_WINDOW
}
fn advance(reference: &mut u32, value: u32) {
    if value.wrapping_sub(*reference) <= SEQUENCE_WINDOW {
        *reference = value;
    }
}

#[derive(Default)]
struct SnapshotBudget {
    window: Duration,
    last: Duration,
    used: u32,
}
impl SnapshotBudget {
    fn take(&mut self, now: Duration, limit: u32) -> Result<(), EvidenceError> {
        if now < self.last {
            return Err(EvidenceError);
        }
        self.last = now;
        if now.saturating_sub(self.window) >= Duration::from_secs(1) {
            self.window = now;
            self.used = 0;
        }
        if self.used >= limit {
            return Err(EvidenceError);
        }
        self.used += 1;
        Ok(())
    }
}

// Microsoft explicitly permits padding between the count and the first row.
// Callers provide offset_of!(official_table_type, table), never sizeof(DWORD).
fn table_extent(bytes: &[u8], offset: usize, row_size: usize) -> Result<usize, EvidenceError> {
    if bytes.len() > MAX_TABLE_BYTES {
        return Err(EvidenceError);
    }
    let count = u32::from_ne_bytes(
        bytes.get(..4).ok_or(EvidenceError)?.try_into().map_err(|_error| EvidenceError)?,
    ) as usize;
    if count > MAX_ROWS || offset < 4 || row_size == 0 {
        return Err(EvidenceError);
    }
    if count == 0 {
        return Ok(0);
    }
    let end = count
        .checked_mul(row_size)
        .and_then(|length| offset.checked_add(length))
        .ok_or(EvidenceError)?;
    if end > bytes.len() {
        return Err(EvidenceError);
    }
    Ok(count)
}

#[cfg(windows)]
mod native {
    use super::*;
    use rokbattles_capture_ipc::windows::{Identity, ProcessIdentity};
    use std::{
        io,
        mem::{offset_of, size_of},
        net::{Ipv4Addr, Ipv6Addr, SocketAddr},
        ptr,
        time::{SystemTime, UNIX_EPOCH},
    };
    use windows_sys::Win32::{
        Foundation::ERROR_INSUFFICIENT_BUFFER,
        NetworkManagement::IpHelper::{
            GetExtendedTcpTable, MIB_TCP6ROW_OWNER_MODULE, MIB_TCP6TABLE_OWNER_MODULE,
            MIB_TCPROW_OWNER_MODULE, MIB_TCPTABLE_OWNER_MODULE, TCP_TABLE_OWNER_MODULE_ALL,
        },
        Networking::WinSock::{AF_INET, AF_INET6},
    };

    // These are SDK ABI checks for the supported Windows x64/ARM64 targets.
    const _: () = {
        assert!(size_of::<MIB_TCPROW_OWNER_MODULE>() == 160);
        assert!(size_of::<MIB_TCP6ROW_OWNER_MODULE>() == 192);
        assert!(offset_of!(MIB_TCPTABLE_OWNER_MODULE, table) == 8);
        assert!(offset_of!(MIB_TCP6TABLE_OWNER_MODULE, table) == 8);
    };

    /// A capture-scoped source. After any error it must be discarded; recreating
    /// it establishes a new baseline before any further packet can be admitted.
    pub struct SocketEvidence {
        state: EvidenceState<ProcessIdentity>,
        owners: ProcessOwners,
        poll_budget: SnapshotBudget,
        packet_budget: SnapshotBudget,
    }
    impl SocketEvidence {
        pub fn new(identity: Identity, generations: SessionGenerations) -> io::Result<Self> {
            let rows = snapshot().map_err(io::Error::other)?;
            // Cutoff AFTER both baseline reads excludes rows racing initialization.
            let cutoff = file_time().map_err(io::Error::other)?;
            let state =
                EvidenceState::baseline(rows, cutoff, generations).map_err(io::Error::other)?;
            Ok(Self {
                state,
                owners: ProcessOwners { identity },
                poll_budget: SnapshotBudget::default(),
                packet_budget: SnapshotBudget::default(),
            })
        }
        pub fn poll(&mut self, now: Duration) -> Result<Vec<EvidenceEvent>, EvidenceError> {
            self.refresh(now, None)
        }
        pub fn server(
            &mut self,
            packet: &ServerPacket<'_>,
            now: Duration,
        ) -> Result<ServerDecision, EvidenceError> {
            let events = self.refresh(now, Some(packet.flow_key()))?;
            self.state.server(packet, now, events)
        }
        fn refresh(
            &mut self,
            now: Duration,
            release: Option<FlowKey>,
        ) -> Result<Vec<EvidenceEvent>, EvidenceError> {
            let result = (|| {
                if self.state.failed {
                    return Err(EvidenceError);
                }
                if release.is_some() {
                    self.packet_budget.take(now, MAX_PACKETS_PER_SECOND)?;
                } else {
                    self.poll_budget.take(now, MAX_POLLS_PER_SECOND)?;
                }
                let rows = snapshot()?;
                let wall_time = file_time()?;
                match release {
                    Some(key) => {
                        self.state.observe_for(rows, wall_time, now, &mut self.owners, Some(key))
                    }
                    None => self.state.observe(rows, wall_time, now, &mut self.owners),
                }
            })();
            if result.is_err() {
                self.state.poison();
            }
            result
        }
    }

    struct ProcessOwners {
        identity: Identity,
    }
    impl Owners for ProcessOwners {
        type Pin = ProcessIdentity;
        fn pin(&mut self, row: Row) -> Result<Option<Self::Pin>, EvidenceError> {
            let pin = ProcessIdentity::open(row.pid).map_err(|_error| EvidenceError)?;
            Ok((pin.identity() == &self.identity
                && pin.is_alive()
                && pin.creation_time() <= row.created)
                .then_some(pin))
        }
        fn alive(&self, pin: &Self::Pin) -> bool {
            pin.is_alive()
        }
        fn matches(&mut self, pin: &Self::Pin, row: Row) -> Result<bool, EvidenceError> {
            if !pin.is_alive()
                || pin.identity() != &self.identity
                || pin.creation_time() > row.created
            {
                return Ok(false);
            }
            // Opening again queries the CURRENT primary token, not only cached
            // TokenUser/logon SIDs. The first handle remains pinned throughout.
            let current = ProcessIdentity::open(row.pid).map_err(|_error| EvidenceError)?;
            Ok(current.creation_time() == pin.creation_time()
                && current.identity() == pin.identity()
                && current.is_alive()
                && pin.is_alive())
        }
    }

    fn file_time() -> Result<u64, EvidenceError> {
        // FILETIME uses 100-ns ticks since 1601; SystemTime uses the same OS wall
        // clock and avoids introducing a hand-written Windows time ABI.
        const UNIX_EPOCH_FILETIME: u64 = 116_444_736_000_000_000;
        let elapsed =
            SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_error| EvidenceError)?;
        let ticks = elapsed
            .as_secs()
            .checked_mul(10_000_000)
            .and_then(|ticks| ticks.checked_add(u64::from(elapsed.subsec_nanos() / 100)))
            .and_then(|ticks| ticks.checked_add(UNIX_EPOCH_FILETIME))
            .ok_or(EvidenceError)?;
        Ok(ticks)
    }

    fn table(af: u32) -> Result<(Vec<u64>, usize), EvidenceError> {
        let mut length = 0;
        // SAFETY: null output performs the documented size query; writable DWORD.
        let status = unsafe {
            GetExtendedTcpTable(ptr::null_mut(), &mut length, 0, af, TCP_TABLE_OWNER_MODULE_ALL, 0)
        };
        if status != ERROR_INSUFFICIENT_BUFFER {
            return Err(EvidenceError);
        }
        // At most three allocations/reads if table growth races a size query.
        for _ in 0..3 {
            if length < 4 || length as usize > MAX_TABLE_BYTES {
                return Err(EvidenceError);
            }
            let capacity = length as usize;
            let mut buffer = vec![0u64; capacity.div_ceil(size_of::<u64>())];
            // SAFETY: u64 storage has the official table/row alignment on both
            // supported Windows architectures and is at least length bytes long.
            let status = unsafe {
                GetExtendedTcpTable(
                    buffer.as_mut_ptr().cast(),
                    &mut length,
                    0,
                    af,
                    TCP_TABLE_OWNER_MODULE_ALL,
                    0,
                )
            };
            if status == 0 {
                if length as usize > capacity || length < 4 {
                    return Err(EvidenceError);
                }
                return Ok((buffer, length as usize));
            }
            if status != ERROR_INSUFFICIENT_BUFFER || length as usize <= capacity {
                return Err(EvidenceError);
            }
        }
        Err(EvidenceError)
    }

    fn snapshot() -> Result<Vec<Row>, EvidenceError> {
        let mut rows = Vec::new();
        let (buffer, length) = table(u32::from(AF_INET))?;
        // SAFETY: OS initialized bytes in uniquely owned buffer, bounded by table().
        let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), length) };
        let offset = offset_of!(MIB_TCPTABLE_OWNER_MODULE, table);
        let count = table_extent(bytes, offset, size_of::<MIB_TCPROW_OWNER_MODULE>())?;
        rows.reserve(count);
        for index in 0..count {
            let start = offset + index * size_of::<MIB_TCPROW_OWNER_MODULE>();
            let row_bytes = bytes
                .get(start..start + size_of::<MIB_TCPROW_OWNER_MODULE>())
                .ok_or(EvidenceError)?;
            // SAFETY: exact checked row extent; every field is an integer and the
            // official windows-sys ABI provides padding/stride on x64 and ARM64.
            let raw = unsafe {
                ptr::read_unaligned(row_bytes.as_ptr().cast::<MIB_TCPROW_OWNER_MODULE>())
            };
            rows.push(Row {
                key: FlowKey {
                    client: SocketAddr::new(
                        Ipv4Addr::from(raw.dwLocalAddr.to_ne_bytes()).into(),
                        port(raw.dwLocalPort)?,
                    ),
                    server: SocketAddr::new(
                        Ipv4Addr::from(raw.dwRemoteAddr.to_ne_bytes()).into(),
                        port(raw.dwRemotePort)?,
                    ),
                },
                pid: raw.dwOwningPid,
                created: u64::try_from(raw.liCreateTimestamp).map_err(|_error| EvidenceError)?,
                state: raw.dwState,
            });
        }
        let (buffer, length) = table(u32::from(AF_INET6))?;
        // SAFETY: same checked owned OS output as the IPv4 table above.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), length) };
        let offset = offset_of!(MIB_TCP6TABLE_OWNER_MODULE, table);
        let count = table_extent(bytes, offset, size_of::<MIB_TCP6ROW_OWNER_MODULE>())?;
        if rows.len().checked_add(count).ok_or(EvidenceError)? > MAX_ROWS {
            return Err(EvidenceError);
        }
        rows.reserve(count);
        for index in 0..count {
            let start = offset + index * size_of::<MIB_TCP6ROW_OWNER_MODULE>();
            let row_bytes = bytes
                .get(start..start + size_of::<MIB_TCP6ROW_OWNER_MODULE>())
                .ok_or(EvidenceError)?;
            // SAFETY: checked row extent and official all-integer ABI.
            let raw = unsafe {
                ptr::read_unaligned(row_bytes.as_ptr().cast::<MIB_TCP6ROW_OWNER_MODULE>())
            };
            // Raw IP capture has no interface scope. Never erase scope and risk
            // authorizing a same-address connection on a different interface.
            if raw.dwLocalScopeId != 0 || raw.dwRemoteScopeId != 0 {
                continue;
            }
            rows.push(Row {
                key: FlowKey {
                    client: SocketAddr::new(
                        Ipv6Addr::from(raw.ucLocalAddr).into(),
                        port(raw.dwLocalPort)?,
                    ),
                    server: SocketAddr::new(
                        Ipv6Addr::from(raw.ucRemoteAddr).into(),
                        port(raw.dwRemotePort)?,
                    ),
                },
                pid: raw.dwOwningPid,
                created: u64::try_from(raw.liCreateTimestamp).map_err(|_error| EvidenceError)?,
                state: raw.dwState,
            });
        }
        Ok(rows)
    }
    fn port(raw: u32) -> Result<u16, EvidenceError> {
        Ok(u16::from_be(u16::try_from(raw).map_err(|_error| EvidenceError)?))
    }
}
#[cfg(windows)]
pub use native::SocketEvidence;

#[cfg(test)]
mod tests {
    use super::*;
    use rokbattles_capture_runtime::packet;

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Pin {
        pid: u32,
        process_created: u64,
        user: u32,
        logon: u32,
    }
    struct MockOwners {
        current: Pin,
        alive: bool,
        unavailable: bool,
        checks: usize,
    }
    impl Default for MockOwners {
        fn default() -> Self {
            Self {
                current: Pin { pid: 77, process_created: 500, user: 1, logon: 2 },
                alive: true,
                unavailable: false,
                checks: 0,
            }
        }
    }
    impl Owners for MockOwners {
        type Pin = Pin;
        fn pin(&mut self, row: Row) -> Result<Option<Pin>, EvidenceError> {
            if self.unavailable {
                return Err(EvidenceError);
            }
            Ok((row.pid == self.current.pid
                && self.current.user == 1
                && self.current.logon == 2
                && self.alive
                && self.current.process_created <= row.created)
                .then_some(self.current))
        }
        fn alive(&self, pin: &Pin) -> bool {
            self.alive
                && pin.pid == self.current.pid
                && pin.process_created == self.current.process_created
        }
        fn matches(&mut self, pin: &Pin, row: Row) -> Result<bool, EvidenceError> {
            self.checks += 1;
            if self.unavailable {
                return Err(EvidenceError);
            }
            Ok(self.alive
                && *pin == self.current
                && pin.pid == row.pid
                && pin.process_created <= row.created)
        }
    }
    fn key(port: u16) -> FlowKey {
        FlowKey {
            client: format!("192.0.2.2:{port}").parse().expect("client"),
            server: "198.51.100.1:3101".parse().expect("server"),
        }
    }
    fn row(port: u16, state: u32) -> Row {
        Row { key: key(port), pid: 77, created: 2000, state }
    }
    fn state() -> EvidenceState<Pin> {
        EvidenceState::baseline(Vec::new(), 1000, SessionGenerations::default()).expect("baseline")
    }
    fn bytes(port: u16, flags: u8, sequence: u32, ack: u32, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x45, 0];
        bytes.extend(u16::try_from(40 + payload.len()).expect("test size").to_be_bytes());
        bytes.extend([0, 0, 0, 0, 64, 6, 0, 0, 198, 51, 100, 1, 192, 0, 2, 2]);
        bytes.extend(3101u16.to_be_bytes());
        bytes.extend(port.to_be_bytes());
        bytes.extend(sequence.to_be_bytes());
        bytes.extend(ack.to_be_bytes());
        bytes.extend([0x50, flags, 0, 0, 0, 0, 0, 0]);
        bytes.extend(payload);
        bytes
    }
    fn observe(
        state: &mut EvidenceState<Pin>,
        rows: Vec<Row>,
        millis: u64,
        owners: &mut MockOwners,
    ) -> Vec<EvidenceEvent> {
        state
            .observe_for(rows, 3000, Duration::from_millis(millis), owners, Some(key(45000)))
            .expect("snapshot")
    }
    fn server(
        state: &mut EvidenceState<Pin>,
        port: u16,
        flags: u8,
        sequence: u32,
        ack: u32,
        payload: &[u8],
        millis: u64,
    ) -> ServerDecision {
        let bytes = bytes(port, flags, sequence, ack, payload);
        state
            .server(
                &packet::parse(&bytes).expect("packet"),
                Duration::from_millis(millis),
                Vec::new(),
            )
            .expect("decision")
    }
    fn connect(state: &mut EvidenceState<Pin>, owners: &mut MockOwners) -> u64 {
        assert!(observe(state, vec![row(45000, SYN_SENT)], 0, owners).is_empty());
        assert!(!server(state, 45000, 0x12, 200, 101, &[], 0).allowed);
        let events = observe(state, vec![row(45000, ESTABLISHED)], 10, owners);
        assert_eq!(events.len(), 1);
        let evidence = events
            .iter()
            .find_map(|event| match event {
                EvidenceEvent::Established(evidence) => Some(evidence),
                EvidenceEvent::Retired(_) => None,
            })
            .expect("established evidence");
        assert_eq!(evidence.client_initial_sequence, 100);
        assert_eq!(evidence.server_initial_sequence, 200);
        evidence.generation
    }

    #[test]
    fn baseline_and_preexisting_bindings_never_become_eligible() {
        let mut owners = MockOwners::default();
        for initial in [SYN_SENT, ESTABLISHED, FIN_WAIT1, TIME_WAIT] {
            let mut state = EvidenceState::baseline(
                vec![row(45000, initial)],
                1000,
                SessionGenerations::default(),
            )
            .expect("baseline");
            assert!(observe(&mut state, vec![row(45000, SYN_SENT)], 0, &mut owners).is_empty());
            assert!(!server(&mut state, 45000, 0x12, 200, 101, &[], 0).allowed);
            assert!(observe(&mut state, vec![row(45000, ESTABLISHED)], 10, &mut owners).is_empty());
            assert!(!server(&mut state, 45000, 0x18, 201, 101, b"private", 10).allowed);
        }
        for created in [0, 500, 1000, 3001] {
            let mut state = state();
            let mut old = row(45000, SYN_SENT);
            old.created = created;
            assert!(observe(&mut state, vec![old], 0, &mut owners).is_empty());
            assert!(state.flows.is_empty());
        }
    }

    #[test]
    fn missed_transitions_and_unseen_server_handshake_fail_closed() {
        let mut owners = MockOwners::default();
        let mut missed = state();
        assert!(observe(&mut missed, vec![row(45000, ESTABLISHED)], 0, &mut owners).is_empty());
        assert!(!server(&mut missed, 45000, 0x12, 200, 101, &[], 0).allowed);
        assert!(missed.flows.is_empty());
        let mut no_synack = state();
        observe(&mut no_synack, vec![row(45000, SYN_SENT)], 0, &mut owners);
        assert!(observe(&mut no_synack, vec![row(45000, ESTABLISHED)], 10, &mut owners).is_empty());
        assert!(!server(&mut no_synack, 45000, 0x18, 201, 101, b"private", 10).allowed);
        assert!(no_synack.flows.is_empty());
    }

    #[test]
    fn complete_witness_revalidates_identity_before_each_packet() {
        let mut owners = MockOwners::default();
        let mut state = state();
        assert_ne!(connect(&mut state, &mut owners), 0);
        let checks = owners.checks;
        assert!(observe(&mut state, vec![row(45000, ESTABLISHED)], 11, &mut owners).is_empty());
        assert!(owners.checks > checks);
        assert!(server(&mut state, 45000, 0x18, 201, 101, b"server only", 11).allowed);
        owners.current.logon = 3;
        assert!(matches!(
            observe(&mut state, vec![row(45000, ESTABLISHED)], 12, &mut owners).as_slice(),
            [EvidenceEvent::Retired(_)]
        ));
        assert!(!server(&mut state, 45000, 0x18, 212, 101, b"other logon", 12).allowed);
    }

    #[test]
    fn same_pid_new_process_new_row_or_changed_user_retires_original_generation() {
        for mutation in 0..4 {
            let mut owners = MockOwners::default();
            let mut state = state();
            let generation = connect(&mut state, &mut owners);
            let mut changed = row(45000, ESTABLISHED);
            match mutation {
                0 => owners.current.process_created = 600,
                1 => owners.current.user = 3,
                2 => changed.created = 2001,
                _ => changed.pid = 78,
            }
            let events = observe(&mut state, vec![changed], 11, &mut owners);
            assert!(
                matches!(events.as_slice(), [EvidenceEvent::Retired(evidence)] if evidence.generation == generation)
            );
            assert!(!server(&mut state, 45000, 0x18, 201, 101, b"stale", 11).allowed);
        }
    }

    #[test]
    fn foreign_user_or_logon_is_never_pinned() {
        for mismatch in 0..3 {
            let mut owners = MockOwners::default();
            let mut state = state();
            match mismatch {
                0 => owners.current.user = 9,
                1 => owners.current.logon = 9,
                _ => owners.alive = false,
            }
            observe(&mut state, vec![row(45000, SYN_SENT)], 0, &mut owners);
            assert!(state.flows.is_empty());
            assert!(!server(&mut state, 45000, 0x12, 200, 101, &[], 0).allowed);
        }
    }

    #[test]
    fn probe_disappearance_retires_only_its_own_tuple() {
        let mut owners = MockOwners::default();
        let mut state = state();
        observe(&mut state, vec![row(45000, SYN_SENT), row(45001, SYN_SENT)], 0, &mut owners);
        server(&mut state, 45000, 0x12, 200, 101, &[], 0);
        server(&mut state, 45001, 0x12, 400, 301, &[], 0);
        let events = observe(
            &mut state,
            vec![row(45000, ESTABLISHED), row(45001, ESTABLISHED)],
            10,
            &mut owners,
        );
        assert_eq!(events.len(), 2);
        let events = observe(&mut state, vec![row(45001, ESTABLISHED)], 11, &mut owners);
        assert!(
            matches!(events.as_slice(), [EvidenceEvent::Retired(evidence)] if evidence.key == key(45000))
        );
        assert!(server(&mut state, 45001, 0x18, 401, 301, b"game", 11).allowed);
        assert!(!server(&mut state, 45000, 0x18, 201, 101, b"late probe", 11).allowed);
    }

    #[test]
    fn fin_wait_keeps_server_tail_but_closed_and_time_wait_retire() {
        for terminal in [CLOSED, TIME_WAIT, DELETE_TCB] {
            let mut owners = MockOwners::default();
            let mut state = state();
            connect(&mut state, &mut owners);
            assert!(observe(&mut state, vec![row(45000, FIN_WAIT1)], 11, &mut owners).is_empty());
            assert!(server(&mut state, 45000, 0x18, 201, 101, b"tail", 11).allowed);
            assert!(observe(&mut state, vec![row(45000, FIN_WAIT2)], 12, &mut owners).is_empty());
            assert!(server(&mut state, 45000, 0x11, 205, 101, &[], 12).allowed);
            assert!(matches!(
                observe(&mut state, vec![row(45000, terminal)], 13, &mut owners).as_slice(),
                [EvidenceEvent::Retired(_)]
            ));
            assert!(!server(&mut state, 45000, 0x18, 206, 101, b"late", 13).allowed);
        }
    }

    #[test]
    fn changed_isns_payload_synack_and_invalid_sequences_abort_only_that_flow() {
        for (flags, sequence, ack, payload) in [
            (0x12, 201, 101, &b""[..]),
            (0x12, 200, 102, &b""[..]),
            (0x12, 200, 101, &b"payload"[..]),
            (0x18, 0x80000000, 101, &b"bad"[..]),
            (0x18, 201, 0x80000000, &b"bad"[..]),
        ] {
            let mut owners = MockOwners::default();
            let mut state = state();
            connect(&mut state, &mut owners);
            observe(&mut state, vec![row(45000, ESTABLISHED)], 11, &mut owners);
            let result = server(&mut state, 45000, flags, sequence, ack, payload, 11);
            assert!(!result.allowed);
            assert!(matches!(result.events.as_slice(), [EvidenceEvent::Retired(_)]));
        }
    }

    #[test]
    fn retransmitted_synack_and_wrapped_isns_do_not_create_another_generation() {
        let mut owners = MockOwners::default();
        let mut state = state();
        observe(&mut state, vec![row(45000, SYN_SENT)], 0, &mut owners);
        assert!(!server(&mut state, 45000, 0x12, u32::MAX, 0, &[], 0).allowed);
        let events = observe(&mut state, vec![row(45000, ESTABLISHED)], 10, &mut owners);
        assert!(
            matches!(events.as_slice(), [EvidenceEvent::Established(e)] if e.client_initial_sequence == u32::MAX && e.server_initial_sequence == u32::MAX)
        );
        assert!(server(&mut state, 45000, 0x12, u32::MAX, 0, &[], 10).events.is_empty());
        assert!(server(&mut state, 45000, 0x18, 0, 0, b"wrapped", 10).allowed);
    }

    #[test]
    fn table_and_identity_uncertainty_poison_the_source() {
        let mut owners = MockOwners::default();
        let mut state = state();
        connect(&mut state, &mut owners);
        owners.unavailable = true;
        state
            .observe_for(
                vec![row(45000, ESTABLISHED)],
                3000,
                Duration::from_millis(11),
                &mut owners,
                Some(key(45000)),
            )
            .expect_err("evidence must fail closed");
        assert!(state.failed && state.flows.is_empty());
        owners.unavailable = false;
        state
            .observe(Vec::new(), 3000, Duration::from_millis(12), &mut owners)
            .expect_err("evidence must fail closed");
        let mut duplicate = super::tests::state();
        duplicate
            .observe(vec![row(45000, SYN_SENT); 2], 3000, Duration::ZERO, &mut owners)
            .expect_err("evidence must fail closed");
        let mut large = super::tests::state();
        large
            .observe(vec![row(45000, SYN_SENT); MAX_ROWS + 1], 3000, Duration::ZERO, &mut owners)
            .expect_err("evidence must fail closed");
    }

    #[test]
    fn provisional_flows_and_poll_work_are_bounded() {
        let mut owners = MockOwners::default();
        let mut state = state();
        let rows = (40000..40000 + MAX_FLOWS as u16).map(|port| row(port, SYN_SENT)).collect();
        observe(&mut state, rows, 0, &mut owners);
        assert_eq!(state.flows.len(), MAX_FLOWS);
        let rows = (40000..40001 + MAX_FLOWS as u16).map(|port| row(port, SYN_SENT)).collect();
        state
            .observe(rows, 3000, Duration::from_millis(10), &mut owners)
            .expect_err("evidence must fail closed");
        assert!(state.failed && state.flows.is_empty());
        let mut budget = SnapshotBudget::default();
        for _ in 0..MAX_POLLS_PER_SECOND {
            budget.take(Duration::ZERO, MAX_POLLS_PER_SECOND).expect("within budget");
        }
        budget.take(Duration::ZERO, MAX_POLLS_PER_SECOND).expect_err("evidence must fail closed");
        let mut packets = SnapshotBudget::default();
        for _ in 0..MAX_PACKETS_PER_SECOND {
            packets
                .take(Duration::ZERO, MAX_PACKETS_PER_SECOND)
                .expect("independent packet budget");
        }
        packets.take(Duration::ZERO, MAX_PACKETS_PER_SECOND).expect_err("packet budget bound");
        budget.take(Duration::from_secs(1), MAX_POLLS_PER_SECOND).expect("within budget");
        budget.take(Duration::ZERO, MAX_POLLS_PER_SECOND).expect_err("evidence must fail closed");
    }

    #[test]
    fn no_output_polls_use_pinned_liveness_and_release_rechecks_the_current_token() {
        let mut owners = MockOwners::default();
        let mut state = state();
        connect(&mut state, &mut owners);
        let checks = owners.checks;
        owners.current.logon = 99;
        let events = state
            .observe(vec![row(45000, ESTABLISHED)], 3000, Duration::from_millis(11), &mut owners)
            .expect("no-output poll");
        assert!(events.is_empty());
        assert_eq!(owners.checks, checks);
        let events = state
            .observe_for(
                vec![row(45000, ESTABLISHED)],
                3000,
                Duration::from_millis(12),
                &mut owners,
                Some(key(45000)),
            )
            .expect("packet revalidation");
        assert!(owners.checks > checks);
        assert!(matches!(events.as_slice(), [EvidenceEvent::Retired(_)]));
        assert!(!server(&mut state, 45000, 0x18, 201, 101, b"other logon", 12).allowed);
    }

    #[test]
    fn provisional_timeout_and_backwards_clocks_fail_closed() {
        let mut owners = MockOwners::default();
        let mut state = state();
        observe(&mut state, vec![row(45000, SYN_SENT)], 0, &mut owners);
        observe(&mut state, vec![row(45000, SYN_SENT)], 30_001, &mut owners);
        assert!(state.flows.is_empty());
        state
            .observe(Vec::new(), 3000, Duration::ZERO, &mut owners)
            .expect_err("evidence must fail closed");
        let mut state = super::tests::state();
        state
            .observe(Vec::new(), 999, Duration::ZERO, &mut owners)
            .expect_err("evidence must fail closed");
    }

    #[test]
    fn padded_native_table_bounds_reject_truncation_overflow_and_oversized_counts() {
        let mut bytes = vec![0u8; 8 + 160];
        bytes.get_mut(..4).expect("count").copy_from_slice(&1u32.to_ne_bytes());
        assert_eq!(table_extent(&bytes, 8, 160), Ok(1));
        for length in 0..bytes.len() {
            table_extent(bytes.get(..length).expect("prefix"), 8, 160)
                .expect_err("evidence must fail closed");
        }
        table_extent(&bytes, usize::MAX, 160).expect_err("evidence must fail closed");
        table_extent(&bytes, 8, usize::MAX).expect_err("evidence must fail closed");
        bytes.get_mut(..4).expect("count").copy_from_slice(&u32::MAX.to_ne_bytes());
        table_extent(&bytes, 8, 160).expect_err("evidence must fail closed");
        assert_eq!(table_extent(&0u32.to_ne_bytes(), 8, 160), Ok(0));
    }

    #[test]
    fn reset_retires_before_any_late_bytes_can_be_admitted() {
        let mut owners = MockOwners::default();
        let mut state = state();
        connect(&mut state, &mut owners);
        observe(&mut state, vec![row(45000, ESTABLISHED)], 11, &mut owners);
        let reset = server(&mut state, 45000, 0x14, 201, 101, &[], 11);
        assert!(!reset.allowed);
        assert!(matches!(reset.events.as_slice(), [EvidenceEvent::Retired(_)]));
        observe(&mut state, vec![row(45000, ESTABLISHED)], 12, &mut owners);
        assert!(!server(&mut state, 45000, 0x18, 201, 101, b"late", 12).allowed);
    }

    #[test]
    fn generations_increase_in_emission_order_and_survive_source_recreation() {
        let mut owners = MockOwners::default();
        let mut original = state();
        observe(&mut original, vec![row(45000, SYN_SENT), row(45001, SYN_SENT)], 0, &mut owners);
        server(&mut original, 45001, 0x12, 400, 301, &[], 0);
        let first = observe(
            &mut original,
            vec![row(45000, SYN_SENT), row(45001, ESTABLISHED)],
            10,
            &mut owners,
        );
        let first = match first.as_slice() {
            [EvidenceEvent::Established(e)] => e.generation,
            _ => 0,
        };
        assert_ne!(first, 0);
        server(&mut original, 45000, 0x12, 200, 101, &[], 10);
        let second = observe(
            &mut original,
            vec![row(45000, ESTABLISHED), row(45001, ESTABLISHED)],
            11,
            &mut owners,
        );
        let second = match second.as_slice() {
            [EvidenceEvent::Established(e)] => e.generation,
            _ => 0,
        };
        assert!(second > first);
        let mut recreated = EvidenceState::baseline(Vec::new(), 1000, original.generations.clone())
            .expect("same session");
        assert!(connect(&mut recreated, &mut owners) > second);
        let mut other_logon = state();
        assert_eq!(connect(&mut other_logon, &mut owners), 1);
    }

    #[test]
    fn debug_output_never_discloses_tuple_or_isns() {
        let mut owners = MockOwners::default();
        let mut state = state();
        observe(&mut state, vec![row(45000, SYN_SENT)], 0, &mut owners);
        server(&mut state, 45000, 0x12, 200, 101, &[], 0);
        let events = observe(&mut state, vec![row(45000, ESTABLISHED)], 10, &mut owners);
        let debug = format!("{events:?}");
        for value in ["192.0.2.2", "198.51.100.1", "45000", "200", "100"] {
            assert!(!debug.contains(value));
        }
    }
}
