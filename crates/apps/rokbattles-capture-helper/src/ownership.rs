//! User/flow attribution before any captured byte crosses the privilege boundary.
//! A TCP-table row alone never authorizes an inbound packet. An observed outbound
//! SYN binds a pinned OS process, then both handshake ISNs identify its generation.
use rokbattles_capture_ipc::{PacketBytes, Record};
use rokbattles_capture_runtime::packet::{self, ClientTcpControl, FlowKey};
use std::{collections::BTreeMap, time::Duration};

const MAX_FLOWS: usize = 128;
const MAX_SYN_LOOKUPS_PER_SECOND: u32 = 32;
const HANDSHAKE_IDLE: Duration = Duration::from_secs(30);
const FLOW_IDLE: Duration = Duration::from_secs(30 * 60);
const SEQUENCE_WINDOW: u32 = 4 * 1024 * 1024;

/// Implementations return only a live pinned process belonging to the already
/// authenticated IPC user and logon. Ambiguous/unavailable lookup must return Err.
pub trait OwnerLookup {
    type Owner;
    fn owner_for_syn(&mut self, key: FlowKey) -> Result<Option<Self::Owner>, OwnershipError>;
    fn alive(&self, owner: &Self::Owner) -> bool;
    fn still_owner(&mut self, key: FlowKey, owner: &Self::Owner) -> bool;
}
#[derive(Debug, Clone, Copy)]
pub struct OwnershipError;

struct BoundFlow<O> {
    owner: O,
    client_isn: u32,
    client_next: u32,
    server_isn: Option<u32>,
    server_next: u32,
    established: bool,
    touched: Duration,
}

pub struct FlowGuard<L: OwnerLookup> {
    lookup: L,
    flows: BTreeMap<FlowKey, BoundFlow<L::Owner>>,
    budget_second: u64,
    lookups: u32,
}
impl<L: OwnerLookup> FlowGuard<L> {
    pub fn new(lookup: L) -> Self {
        Self { lookup, flows: BTreeMap::new(), budget_second: 0, lookups: 0 }
    }
    pub fn gap(&mut self) -> Record {
        self.flows.clear();
        Record::Gap
    }

    /// An unknown/foreign SYN discloses no metadata. Generation replacement or
    /// ownership uncertainty invalidates all downstream state before continuing.
    pub fn client(&mut self, control: ClientTcpControl, now: Duration) -> Vec<Record> {
        let mut records = self.expire(now);
        if !control.is_valid() {
            records.push(self.gap());
            return records;
        }
        let flags = control.flags & 0x3f;
        if flags == 0x02 {
            if let Some(flow) = self.flows.get_mut(&control.key) {
                if flow.client_isn == control.sequence
                    && self.lookup.still_owner(control.key, &flow.owner)
                {
                    flow.touched = now;
                    records.push(Record::ClientControl(control));
                    return records;
                }
                records.push(self.gap());
            }
            if self.budget_second != now.as_secs() {
                self.budget_second = now.as_secs();
                self.lookups = 0;
            }
            if self.lookups >= MAX_SYN_LOOKUPS_PER_SECOND || self.flows.len() >= MAX_FLOWS {
                records.push(self.gap());
                return records;
            }
            self.lookups += 1;
            match self.lookup.owner_for_syn(control.key) {
                Ok(Some(owner)) if self.lookup.alive(&owner) => {
                    self.flows.insert(
                        control.key,
                        BoundFlow {
                            owner,
                            client_isn: control.sequence,
                            client_next: control.sequence.wrapping_add(1),
                            server_isn: None,
                            server_next: 0,
                            established: false,
                            touched: now,
                        },
                    );
                    records.push(Record::ClientControl(control));
                }
                Ok(None) => {}
                _ => records.push(self.gap()),
            }
            return records;
        }
        let Some(flow) = self.flows.get_mut(&control.key) else {
            return records;
        };
        if !self.lookup.still_owner(control.key, &flow.owner) {
            records.push(self.gap());
            return records;
        }
        let Some(server_isn) = flow.server_isn else {
            return records;
        };
        if !flow.established {
            if flags != 0x10
                || control.sequence != flow.client_isn.wrapping_add(1)
                || control.acknowledgement != server_isn.wrapping_add(1)
            {
                return records;
            }
            flow.established = true;
        } else if !near(control.sequence, flow.client_next)
            || (flags & 0x10 != 0 && !near(control.acknowledgement, flow.server_next))
        {
            records.push(self.gap());
            return records;
        }
        advance(&mut flow.client_next, control.sequence);
        flow.touched = now;
        records.push(Record::ClientControl(control));
        records
    }

    pub fn server(&mut self, bytes: impl Into<PacketBytes>, now: Duration) -> Vec<Record> {
        let bytes = bytes.into();
        let mut records = self.expire(now);
        if bytes.len() > rokbattles_capture_ipc::MAX_BODY_BYTES {
            records.push(self.gap());
            return records;
        }
        let Some(packet) = packet::parse(&bytes) else {
            return records;
        };
        let key = packet.flow_key();
        let Some(flow) = self.flows.get_mut(&key) else {
            return records;
        };
        if !self.lookup.still_owner(key, &flow.owner) {
            records.push(self.gap());
            return records;
        }
        let flags = packet.flags() & 0x3f;
        if flags & 0x02 != 0 {
            if flags != 0x12
                || packet.payload_len() != 0
                || packet.acknowledgement() != flow.client_isn.wrapping_add(1)
                || flow.server_isn.is_some_and(|isn| isn != packet.sequence())
            {
                records.push(self.gap());
                return records;
            }
            if flow.server_isn.is_none() {
                flow.server_isn = Some(packet.sequence());
                flow.server_next = packet.sequence().wrapping_add(1);
            }
        } else {
            if !flow.established {
                return records;
            }
            if !near(packet.sequence(), flow.server_next) {
                records.push(self.gap());
                return records;
            }
            let length = packet.payload_len() as u32 + u32::from(flags & 0x01 != 0);
            advance(&mut flow.server_next, packet.sequence().wrapping_add(length));
        }
        flow.touched = now;
        records.push(Record::ServerPacket(bytes));
        records
    }

    /// Recheck immediately before the final IPC write, including after any
    /// awaited metadata/Gap write. Authority never travels through a queue.
    pub fn authorize_record(&mut self, record: &Record) -> bool {
        let key = match record {
            Record::ServerPacket(bytes) => match packet::parse(bytes) {
                Some(packet) => packet.flow_key(),
                None => return false,
            },
            Record::ClientControl(control) => control.key,
            _ => return true,
        };
        self.flows.get(&key).is_some_and(|flow| self.lookup.still_owner(key, &flow.owner))
    }

    /// Retire terminal ownership only after its exact reset record was written.
    /// On write failure the whole IPC session closes and drops the guard instead.
    pub fn record_written(&mut self, record: &Record) {
        let key = match record {
            Record::ClientControl(control) if control.flags & 0x04 != 0 => Some(control.key),
            Record::ServerPacket(bytes) => packet::parse(bytes)
                .filter(|packet| packet.flags() & 0x04 != 0)
                .map(|packet| packet.flow_key()),
            _ => None,
        };
        if let Some(key) = key {
            self.flows.remove(&key);
        }
    }

    pub fn expire(&mut self, now: Duration) -> Vec<Record> {
        // Expiring a binding retires downstream state too; never silently reuse it.
        if self.flows.values().any(|flow| {
            now.saturating_sub(flow.touched)
                > if flow.established { FLOW_IDLE } else { HANDSHAKE_IDLE }
                || !self.lookup.alive(&flow.owner)
        }) {
            vec![self.gap()]
        } else {
            Vec::new()
        }
    }
}
fn near(sequence: u32, reference: u32) -> bool {
    sequence.wrapping_sub(reference) <= SEQUENCE_WINDOW
        || reference.wrapping_sub(sequence) <= SEQUENCE_WINDOW
}
fn advance(reference: &mut u32, sequence: u32) {
    if sequence.wrapping_sub(*reference) <= SEQUENCE_WINDOW {
        *reference = sequence;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Lookup {
        owned: bool,
        uncertain: bool,
        alive: bool,
        calls: usize,
    }
    impl OwnerLookup for Lookup {
        type Owner = ();
        fn owner_for_syn(&mut self, _: FlowKey) -> Result<Option<()>, OwnershipError> {
            self.calls += 1;
            if self.uncertain { Err(OwnershipError) } else { Ok(self.owned.then_some(())) }
        }
        fn alive(&self, _: &()) -> bool {
            self.alive
        }
        fn still_owner(&mut self, _: FlowKey, _: &()) -> bool {
            self.alive && self.owned && !self.uncertain
        }
    }
    fn guard() -> FlowGuard<Lookup> {
        FlowGuard::new(Lookup { owned: true, uncertain: false, alive: true, calls: 0 })
    }
    fn control(flags: u8, sequence: u32, acknowledgement: u32) -> ClientTcpControl {
        ClientTcpControl {
            key: FlowKey {
                client: "192.0.2.2:45000".parse().expect("client"),
                server: "198.51.100.1:3101".parse().expect("server"),
            },
            flags,
            sequence,
            acknowledgement,
        }
    }
    fn packet(flags: u8, sequence: u32, ack: u32, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x45, 0];
        bytes.extend(((40 + payload.len()) as u16).to_be_bytes());
        bytes.extend([0, 0, 0, 0, 64, 6, 0, 0, 198, 51, 100, 1, 192, 0, 2, 2]);
        bytes.extend(3101u16.to_be_bytes());
        bytes.extend(45000u16.to_be_bytes());
        bytes.extend(sequence.to_be_bytes());
        bytes.extend(ack.to_be_bytes());
        bytes.extend([0x50, flags, 0, 0, 0, 0, 0, 0]);
        bytes.extend(payload);
        bytes
    }
    fn handshake(guard: &mut FlowGuard<Lookup>) {
        assert!(matches!(
            guard.client(control(2, 100, 0), Duration::ZERO).as_slice(),
            [Record::ClientControl(_)]
        ));
        assert!(matches!(
            guard.server(packet(0x12, 200, 101, &[]), Duration::ZERO).as_slice(),
            [Record::ServerPacket(_)]
        ));
        assert!(matches!(
            guard.client(control(0x10, 101, 201), Duration::ZERO).as_slice(),
            [Record::ClientControl(_)]
        ));
    }
    #[test]
    fn midstream_foreign_and_incomplete_handshake_never_forward_bytes() {
        let mut g = guard();
        assert!(g.server(packet(0x18, 201, 101, b"private"), Duration::ZERO).is_empty());
        assert_eq!(g.lookup.calls, 0);
        g.lookup.owned = false;
        assert!(g.client(control(2, 100, 0), Duration::ZERO).is_empty());
        assert!(g.server(packet(0x12, 200, 101, &[]), Duration::ZERO).is_empty());
        g.lookup.owned = true;
        g.client(control(2, 100, 0), Duration::ZERO);
        g.server(packet(0x12, 200, 101, &[]), Duration::ZERO);
        assert!(g.server(packet(0x18, 201, 101, b"private"), Duration::ZERO).is_empty());
    }
    #[test]
    fn only_bound_generation_forwards_and_tuple_reuse_emits_gap_first() {
        let mut g = guard();
        handshake(&mut g);
        assert!(matches!(
            g.server(packet(0x18, 201, 101, b"ours"), Duration::ZERO).as_slice(),
            [Record::ServerPacket(_)]
        ));
        assert_eq!(g.lookup.calls, 1);
        assert!(matches!(
            g.client(control(2, 300, 0), Duration::ZERO).as_slice(),
            [Record::Gap, Record::ClientControl(_)]
        ));
        assert!(g.server(packet(0x18, 205, 101, b"old"), Duration::ZERO).is_empty());
        assert!(matches!(
            g.server(packet(0x12, 200, 101, &[]), Duration::ZERO).as_slice(),
            [Record::Gap]
        ));
    }
    #[test]
    fn process_exit_lookup_failure_and_idle_expiry_invalidate_every_flow() {
        let mut g = guard();
        handshake(&mut g);
        g.lookup.alive = false;
        assert!(matches!(
            g.server(packet(0x18, 201, 101, b"private"), Duration::ZERO).as_slice(),
            [Record::Gap]
        ));
        g.lookup.alive = true;
        g.lookup.uncertain = true;
        assert!(matches!(g.client(control(2, 100, 0), Duration::ZERO).as_slice(), [Record::Gap]));
        g.lookup.uncertain = false;
        handshake(&mut g);
        assert!(matches!(g.expire(Duration::from_secs(1801)).as_slice(), [Record::Gap]));
    }
    #[test]
    fn syn_lookup_work_and_memory_are_bounded() {
        let mut g = guard();
        for port in 40000..40100 {
            let mut syn = control(2, 100, 0);
            syn.key.client.set_port(port);
            g.client(syn, Duration::ZERO);
        }
        assert_eq!(g.lookup.calls, MAX_SYN_LOOKUPS_PER_SECOND as usize);
        assert!(g.flows.len() <= MAX_FLOWS);
        assert!(g.server(packet(0x18, 201, 101, b"after loss"), Duration::ZERO).is_empty());
    }
    #[test]
    fn tuple_reassigned_while_old_process_lives_never_forwards_new_user_bytes() {
        let mut g = guard();
        handshake(&mut g);
        g.lookup.owned = false;
        assert!(matches!(
            g.server(packet(0x18, 201, 101, b"other user"), Duration::ZERO).as_slice(),
            [Record::Gap]
        ));
        assert!(g.server(packet(0x18, 201, 101, b"other user"), Duration::ZERO).is_empty());
        g.lookup.owned = true;
        handshake(&mut g);
        g.lookup.owned = false;
        assert!(matches!(
            g.client(control(0x10, 101, 201), Duration::ZERO).as_slice(),
            [Record::Gap]
        ));
    }
    #[test]
    fn client_fin_keeps_the_owned_server_tail_and_writer_rechecks_authority() {
        let mut g = guard();
        handshake(&mut g);
        let records = g.client(control(0x11, 101, 201), Duration::ZERO);
        assert!(matches!(records.as_slice(), [Record::ClientControl(_)]));
        let tail = g.server(packet(0x18, 201, 102, b"server tail"), Duration::ZERO);
        let record = tail.first().expect("tail");
        assert!(g.authorize_record(record));
        g.lookup.owned = false;
        assert!(!g.authorize_record(record));
    }
    #[test]
    fn reset_authority_survives_until_exact_record_written_then_retires() {
        for server_reset in [false, true] {
            let mut g = guard();
            handshake(&mut g);
            let records = if server_reset {
                g.server(packet(0x14, 201, 101, &[]), Duration::ZERO)
            } else {
                g.client(control(0x14, 101, 201), Duration::ZERO)
            };
            assert_eq!(records.len(), 1);
            let record = records.first().expect("exact reset");
            assert!(!matches!(record, Record::Gap));
            assert!(g.authorize_record(record));
            g.record_written(record);
            assert!(!g.authorize_record(record));
            assert!(g.server(packet(0x18, 201, 101, b"after reset"), Duration::ZERO).is_empty());
        }
    }
}
