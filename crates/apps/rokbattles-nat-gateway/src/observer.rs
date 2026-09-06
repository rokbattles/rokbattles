//! Connects witnessed TCP handshakes to independent server decoders.

use std::{collections::HashMap, net::SocketAddrV4, time::Instant};

use bytes::Bytes;
use rokbattles_gateway_protocol::{
    RuntimeArtifact,
    stream::{ServerStreamProcessor, StreamEvent},
    uploader::{MailBatch, MailContext},
};

use crate::{capture, packet, reassembly::Reassembly};

const MAX_FLOWS: usize = 4096;
const MAX_BUFFERED: usize = 128 * 1024 * 1024;
const MAX_BATCH_BYTES: usize = 24 * 1024 * 1024;
type FlowKey = (SocketAddrV4, SocketAddrV4);
struct Flow<'a> {
    client_next: u32,
    server_start: Option<u32>,
    reassembly: Option<Reassembly>,
    decoder: ServerStreamProcessor<'a>,
    context: MailContext,
    touched: Instant,
}

/// Bounded observation state. It owns no forwarding sockets or kernel verdicts.
pub struct Observer<'a> {
    artifact: &'a RuntimeArtifact,
    flows: HashMap<FlowKey, Flow<'a>>,
    sequence: Option<u32>,
    uploads: std::sync::mpsc::SyncSender<MailBatch>,
}
impl<'a> Observer<'a> {
    pub fn new(
        artifact: &'a RuntimeArtifact,
        uploads: std::sync::mpsc::SyncSender<MailBatch>,
    ) -> Self {
        Self { artifact, flows: HashMap::new(), sequence: None, uploads }
    }

    /// Abandon cipher state after an unknown capture loss. New SYNs may still
    /// start observation; existing TCP sessions continue entirely in the kernel.
    pub fn lost(&mut self, reason: &str) {
        tracing::warn!(
            reason,
            flows = self.flows.len(),
            "capture discontinuity; forwarding remains active"
        );
        self.flows.clear();
        self.sequence = None;
    }

    pub fn record(&mut self, record: capture::Record<'_>) {
        if self.sequence.is_some_and(|sequence| record.sequence != sequence.wrapping_add(1)) {
            self.lost("NFLOG sequence gap");
        }
        self.sequence = Some(record.sequence);
        let Some(packet) = packet::parse(record.payload) else {
            return;
        };
        let key = if record.reply {
            (packet.destination, packet.source)
        } else {
            (packet.source, packet.destination)
        };
        if !record.reply && packet.flags & 0x17 == 0x02 {
            if self
                .flows
                .get(&key)
                .is_some_and(|flow| flow.client_next == packet.sequence.wrapping_add(1))
            {
                return;
            }
            if self.flows.len() >= MAX_FLOWS
                && let Some(oldest) =
                    self.flows.iter().min_by_key(|(_, flow)| flow.touched).map(|(key, _)| *key)
            {
                self.flows.remove(&oldest);
                tracing::warn!(
                    "observation capacity reached; least recently active decoder retired"
                );
            }
            self.flows.insert(
                key,
                Flow {
                    client_next: packet.sequence.wrapping_add(1),
                    server_start: None,
                    reassembly: None,
                    decoder: ServerStreamProcessor::new(self.artifact),
                    context: MailContext::default(),
                    touched: Instant::now(),
                },
            );
            return;
        }
        let Some(flow) = self.flows.get_mut(&key) else {
            return;
        };
        flow.touched = Instant::now();
        if packet.flags & 4 != 0 {
            self.flows.remove(&key);
            return;
        }
        if !record.reply {
            return;
        }
        if packet.flags & 0x12 == 0x12 {
            if packet.acknowledgement != flow.client_next {
                return;
            }
            let start = packet.sequence.wrapping_add(1);
            if flow.server_start.is_some_and(|old| old != start) {
                self.flows.remove(&key);
                return;
            }
            if flow.reassembly.is_none() {
                flow.server_start = Some(start);
                flow.reassembly = Some(Reassembly::new(start));
            }
        }
        let Some(reassembly) = &mut flow.reassembly else {
            return;
        };
        let sequence = packet.sequence.wrapping_add(u32::from(packet.flags & 2 != 0));
        let result = reassembly.push(sequence, packet.payload).and_then(|bytes| {
            flow.decoder.push(&bytes).map_err(|_error| "protocol/cipher decoding failed")
        });
        match result {
            Ok(events) => {
                for event in events {
                    match event {
                        StreamEvent::Login { player_id, server_id } => {
                            flow.context = MailContext {
                                player_id: Some(player_id),
                                server_id: Some(server_id),
                            }
                        }
                        StreamEvent::Mails { server_id, entries, .. } => {
                            if flow.context.server_id.is_none() {
                                flow.context.server_id = server_id.filter(|id| *id != 0);
                            }
                            submit_entries(&self.uploads, &flow.context, entries);
                        }
                    }
                }
            }
            Err(reason) => {
                tracing::warn!(reason, "flow observation retired; forwarding remains active");
                self.flows.remove(&key);
                return;
            }
        }
        if packet.flags & 1 != 0 {
            self.flows.remove(&key);
        }
        let buffered: usize = self
            .flows
            .values()
            .map(|flow| {
                flow.decoder.buffered_bytes()
                    + flow.reassembly.as_ref().map_or(0, Reassembly::buffered)
            })
            .sum();
        if buffered > MAX_BUFFERED {
            self.flows.remove(&key);
            tracing::warn!(
                buffered,
                "global observation memory budget reached; growing decoder retired"
            );
        }
    }
}

fn submit_entries(
    uploads: &std::sync::mpsc::SyncSender<MailBatch>,
    context: &MailContext,
    entries: Vec<Vec<u8>>,
) {
    let mut batch = MailBatch { context: context.clone(), entries: Vec::new() };
    let mut size = 0;
    for entry in entries {
        if entry.len() > MAX_BATCH_BYTES {
            tracing::warn!("mail entry exceeds ingress batch budget; entry skipped");
            continue;
        }
        if size + entry.len() > MAX_BATCH_BYTES || batch.entries.len() >= 512 {
            submit(uploads, &mut batch);
            batch.entries.clear();
            size = 0;
        }
        size += entry.len();
        batch.entries.push(Bytes::from(entry));
    }
    if !batch.entries.is_empty() {
        submit(uploads, &mut batch);
    }
}
fn submit(uploads: &std::sync::mpsc::SyncSender<MailBatch>, batch: &mut MailBatch) {
    let count = batch.entries.len();
    let pending =
        MailBatch { context: batch.context.clone(), entries: std::mem::take(&mut batch.entries) };
    if let Err(error) = uploads.try_send(pending) {
        tracing::warn!(%error, entries = count, "mail batch not queued; decoder remains active");
    }
}

#[cfg(test)]
mod tests {
    use rokbattles_gateway_protocol::stream::test_server_frames;

    use super::*;

    fn packet(reply: bool, seq: u32, ack: u32, flags: u8, payload: &[u8]) -> Vec<u8> {
        let (source, destination, sport, dport) = if reply {
            ([198, 51, 100, 20], [192, 0, 2, 10], 3101_u16, 45000_u16)
        } else {
            ([192, 0, 2, 10], [198, 51, 100, 20], 45000, 3101)
        };
        let mut bytes = vec![0x45, 0];
        bytes.extend(((40 + payload.len()) as u16).to_be_bytes());
        bytes.extend([0, 0, 0, 0, 64, 6, 0, 0]);
        bytes.extend(source);
        bytes.extend(destination);
        bytes.extend(sport.to_be_bytes());
        bytes.extend(dport.to_be_bytes());
        bytes.extend(seq.to_be_bytes());
        bytes.extend(ack.to_be_bytes());
        bytes.extend([0x50, flags, 0xff, 0xff, 0, 0, 0, 0]);
        bytes.extend(payload);
        bytes
    }
    fn feed(
        observer: &mut Observer<'_>,
        counter: &mut u32,
        reply: bool,
        seq: u32,
        ack: u32,
        flags: u8,
        bytes: &[u8],
    ) {
        let payload = packet(reply, seq, ack, flags, bytes);
        observer.record(capture::Record { sequence: *counter, reply, payload: &payload });
        *counter += 1;
    }
    #[test]
    fn full_upload_queue_drops_only_that_batch_and_later_mail_is_uploaded() {
        let artifact = RuntimeArtifact::test_fixture();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 200, 101, 0x12, b"");
        let mut sequence = 201;
        for (index, frame) in test_server_frames(3).iter().enumerate() {
            feed(&mut observer, &mut counter, true, sequence, 101, 0x10, frame);
            sequence += frame.len() as u32;
            if index == 3 {
                receiver.try_recv().expect("first batch filled the queue; second was dropped");
            }
        }
        let batch = receiver.try_recv().expect("third batch still arrives on the same decoder");
        assert_eq!(batch.context, MailContext { player_id: Some(42), server_id: Some(1804) });
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(observer.flows.len(), 1);
    }
    #[test]
    fn capture_gap_never_reuses_cipher_state_and_a_new_handshake_recovers() {
        let artifact = RuntimeArtifact::test_fixture();
        let (sender, receiver) = std::sync::mpsc::sync_channel(8);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 200, 101, 0x12, b"");
        counter += 1;
        for frame in test_server_frames(1) {
            feed(&mut observer, &mut counter, true, 201, 101, 0x10, &frame);
        }
        assert!(matches!(receiver.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        feed(&mut observer, &mut counter, false, 1000, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 2000, 1001, 0x12, b"");
        let mut sequence = 2001;
        for frame in test_server_frames(1) {
            feed(&mut observer, &mut counter, true, sequence, 1001, 0x10, &frame);
            sequence += frame.len() as u32;
        }
        assert_eq!(receiver.try_recv().expect("new flow mail").entries.len(), 1);
    }
    #[test]
    fn encrypted_reordered_segments_and_retransmissions_emit_exactly_one_raw_mail() {
        let artifact = RuntimeArtifact::test_fixture();
        let (sender, receiver) = std::sync::mpsc::sync_channel(8);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 200, 101, 0x12, b"");
        let bytes: Vec<_> = test_server_frames(1).into_iter().flatten().collect();
        let pieces: Vec<_> = bytes.chunks(7).enumerate().collect();
        for (index, bytes) in pieces.iter().rev() {
            feed(&mut observer, &mut counter, true, 201 + (*index as u32 * 7), 101, 0x10, bytes);
        }
        for (index, bytes) in pieces {
            feed(&mut observer, &mut counter, true, 201 + (index as u32 * 7), 101, 0x10, bytes);
        }
        let batch = receiver.try_recv().expect("reassembled mail");
        assert_eq!(batch.entries, vec![Bytes::from_static(b"\x0a\x07mail-id\x4a\x06Battle")]);
        assert!(matches!(receiver.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn long_lived_stream_has_no_cumulative_mail_quota() {
        let artifact = RuntimeArtifact::test_fixture();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 200, 101, 0x12, b"");
        let mut sequence = 201;
        let mut received = 0;
        for frame in test_server_frames(4096) {
            feed(&mut observer, &mut counter, true, sequence, 101, 0x10, &frame);
            sequence += frame.len() as u32;
            if receiver.try_recv().is_ok() {
                received += 1;
            }
        }
        assert_eq!(received, 4096);
    }
}
