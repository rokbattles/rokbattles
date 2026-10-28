//! Associate captured handshakes with per-connection protocol decoders.
//!
//! Flow keys are `(client, server)` socket pairs, normalized with conntrack's
//! reply-direction flag. A client SYN creates state; a matching server SYN/ACK
//! anchors the reassembler. Only server payload is decoded. Client payload is
//! ignored after it establishes the connection's sequence relationship.
//!
//! Capture sequence gaps invalidate every decoder because the missing packet's
//! connection is unknown. TCP overlap, decoding, or memory failures retire only
//! the affected connection. Neither action changes kernel forwarding.
//!
//! Completed mail batches belong to [`Writer`] after submission, independently
//! of the flow. Observation state is bounded, but it has no idle timeout and is
//! not a count of currently connected players.

use std::{net::SocketAddrV4, time::Instant};

use bytes::Bytes;
use rokbattles_gateway_protocol::{
    RuntimeArtifact,
    stream::{ServerStreamProcessor, StreamEvent},
    uploader::{MailBatch, MailContext},
};
use rustc_hash::FxHashMap;

use crate::{capture, packet, reassembly::Reassembly, spool::Writer};

const MAX_FLOWS: usize = 4096;
const MAX_BUFFERED: usize = 128 * 1024 * 1024;
const MAX_BATCH_BYTES: usize = 24 * 1024 * 1024;

// Keep keys in client/server order even though captured packets travel both ways.
type FlowKey = (SocketAddrV4, SocketAddrV4);

/// Cipher state belongs to one witnessed handshake, never just a reused tuple.
struct Flow<'a> {
    /// ACK expected from the server after the client's SYN.
    client_next: u32,
    /// First server payload sequence; absent until a matching SYN/ACK arrives.
    server_start: Option<u32>,
    reassembly: Option<Reassembly>,
    decoder: ServerStreamProcessor<'a>,
    /// Login identity, with the mail carrier's kingdom as a fallback.
    context: MailContext,
    /// Last observed packet time, used for capacity eviction rather than expiry.
    touched: Instant,
}

/// Stateful decoder collection for packet copies from one NFLOG group.
///
/// Retains at most 4096 flows and checks a 128 MiB aggregate decoder/reassembly
/// budget after processing server packets. Flow-capacity eviction uses the least
/// recently touched entry. These limits exclude the separate upload queue.
///
/// No forwarding socket or kernel verdict is owned here. A restart, eviction,
/// or capture gap requires a new handshake before that connection can be decoded.
pub struct Observer<'a> {
    artifact: &'a RuntimeArtifact,
    flows: FxHashMap<FlowKey, Flow<'a>>,
    sequence: Option<u32>,
    uploads: Writer,
}

impl<'a> Observer<'a> {
    /// Create an observer using a validated protocol artifact and spool writer.
    ///
    /// Existing kernel connections have no decoder until a fresh handshake is
    /// captured. The artifact must outlive all flows created by this observer.
    pub fn new(artifact: &'a RuntimeArtifact, uploads: Writer) -> Self {
        Self { artifact, flows: FxHashMap::default(), sequence: None, uploads }
    }

    /// Discard all flow state after a capture discontinuity.
    ///
    /// Logs `reason` and resets capture sequence tracking. An unknown missing
    /// packet may contain cipher bytes for any flow, so retaining individual
    /// decoders would risk decoding corrupt data. Queued/spooled mail is retained.
    pub fn lost(&mut self, reason: &str) {
        tracing::warn!(
            reason,
            flows = self.flows.len(),
            "capture discontinuity; forwarding remains active"
        );
        self.flows.clear();
        self.sequence = None;
    }

    /// Process one ordered NFLOG record, retiring state when it cannot be trusted.
    ///
    /// Checks the capture sequence before parsing the packet, including records
    /// whose IP/TCP bytes will be ignored. Sequence wrap from `u32::MAX` to zero
    /// is contiguous. A gap clears existing state but this record may still start
    /// a new flow if it contains a client SYN.
    ///
    /// Malformed packets, unobserved connections, and client payload are ignored.
    /// A reset retires either direction; a server FIN is processed for payload
    /// before retirement. Queue/disk errors are logged without disabling later
    /// decoding. See [`Writer::try_send`] for its saturation fallback.
    pub fn record(&mut self, record: capture::Record<'_>) {
        if self.sequence.is_some_and(|sequence| record.sequence != sequence.wrapping_add(1)) {
            self.lost("NFLOG sequence gap");
        }
        self.sequence = Some(record.sequence);

        let Some(packet) = packet::parse(record.payload) else {
            return;
        };
        // NAT direction metadata, not an address heuristic, determines the key.
        let key = if record.reply {
            (packet.destination, packet.source)
        } else {
            (packet.source, packet.destination)
        };

        // Retransmitted SYNs must not reset cipher state. A different initial
        // sequence number on the same tuple starts a new connection instead.
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

        // The ACK must match the client's SYN before server bytes can establish
        // a reassembly origin. SYN itself consumes one TCP sequence number.
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
        // A SYN/ACK may carry payload; its first byte follows the SYN's sequence.
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

        // Charge undecoded protocol bytes as well as out-of-order TCP payloads.
        // Evict the flow that just grew rather than disrupting unrelated decoders.
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

/// Split decoder output at ingress's entry-count and aggregate-payload limits.
/// An oversized individual entry is skipped; smaller following entries remain eligible.
fn submit_entries(uploads: &Writer, context: &MailContext, entries: Vec<Vec<u8>>) {
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
// Moving entries out leaves an empty batch with the same context for the next chunk.
fn submit(uploads: &Writer, batch: &mut MailBatch) {
    let count = batch.entries.len();
    let pending =
        MailBatch { context: batch.context.clone(), entries: std::mem::take(&mut batch.entries) };
    if let Err(error) = uploads.try_send(pending) {
        tracing::error!(%error, entries = count, "mail batch could not be queued or committed; decoder remains active");
    }
}

#[cfg(test)]
mod tests {
    use rokbattles_gateway_protocol::stream::test_server_frames;

    use super::*;
    use crate::spool::{QueuedBatch, Spool};

    fn uploads(
        capacity: usize,
    ) -> (tempfile::TempDir, Spool, Writer, std::sync::mpsc::Receiver<QueuedBatch>) {
        let directory = tempfile::tempdir().expect("directory");
        let spool = Spool::open(directory.path().to_path_buf()).expect("spool");
        let (sender, receiver) = Writer::channel(spool.clone(), capacity, 128 * 1024 * 1024);
        (directory, spool, sender, receiver)
    }

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
        *counter = counter.wrapping_add(1);
    }

    #[test]
    fn full_upload_queue_persists_overflow_and_later_mail_is_still_queued() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, spool, sender, receiver) = uploads(1);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 200, 101, 0x12, b"");
        let mut sequence = 201;
        for (index, frame) in test_server_frames(3).iter().enumerate() {
            feed(&mut observer, &mut counter, true, sequence, 101, 0x10, frame);
            sequence += frame.len() as u32;
            if index == 3 {
                receiver.try_recv().expect("first batch filled the queue; second was spooled");
            }
        }
        let batch = receiver
            .try_recv()
            .expect("third batch still arrives on the same decoder")
            .into_batch();
        let paths = spool.pending().expect("pending");
        assert_eq!(paths.len(), 1, "overflow must be durable instead of dropped");
        assert_eq!(Spool::read(&paths[0]).expect("overflow batch").entries.len(), 1);
        assert_eq!(batch.context, MailContext { player_id: Some(42), server_id: Some(1804) });
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(observer.flows.len(), 1);
    }

    #[test]
    fn capture_gap_never_reuses_cipher_state_and_a_new_handshake_recovers() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, receiver) = uploads(8);
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
        assert_eq!(receiver.try_recv().expect("new flow mail").into_batch().entries.len(), 1);
    }

    #[test]
    fn encrypted_reordered_segments_and_retransmissions_emit_exactly_one_raw_mail() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, receiver) = uploads(8);
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
        let batch = receiver.try_recv().expect("reassembled mail").into_batch();
        assert_eq!(batch.entries, vec![Bytes::from_static(b"\x0a\x07mail-id\x4a\x06Battle")]);
        assert!(matches!(receiver.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn long_lived_stream_has_no_cumulative_mail_quota() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, receiver) = uploads(1);
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

    #[test]
    fn capture_sequence_wrap_and_retransmitted_syn_preserve_decoder_state() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, receiver) = uploads(8);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = u32::MAX;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        feed(&mut observer, &mut counter, true, 200, 101, 0x12, b"");

        let mut sequence = 201;
        for (index, frame) in test_server_frames(1).iter().enumerate() {
            feed(&mut observer, &mut counter, true, sequence, 101, 0x10, frame);
            sequence += u32::try_from(frame.len()).expect("frame length");
            if index == 1 {
                feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
            }
        }

        assert_eq!(observer.flows.len(), 1);
        assert_eq!(
            receiver.try_recv().expect("mail after SYN retransmission").into_batch().entries.len(),
            1
        );
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn syn_ack_payload_and_fin_payload_are_decoded_before_retirement() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, receiver) = uploads(8);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
        let frames = test_server_frames(1);
        let mut sequence = 200;
        for (index, frame) in frames.iter().enumerate() {
            let flags = if index == 0 {
                0x12
            } else if index + 1 == frames.len() {
                0x11
            } else {
                0x10
            };
            feed(&mut observer, &mut counter, true, sequence, 101, flags, frame);
            sequence += u32::try_from(frame.len()).expect("frame length") + u32::from(index == 0);
        }

        assert!(observer.flows.is_empty(), "server FIN retires the decoder");
        assert_eq!(
            receiver.try_recv().expect("mail carried with FIN").into_batch().entries.len(),
            1
        );
    }

    #[test]
    fn mismatched_syn_ack_cannot_anchor_a_decoder_and_resets_retire_both_directions() {
        for reply in [false, true] {
            let artifact = RuntimeArtifact::test_fixture();
            let (_directory, _spool, sender, receiver) = uploads(8);
            let mut observer = Observer::new(&artifact, sender);
            let mut counter = 0;
            feed(&mut observer, &mut counter, false, 100, 0, 2, b"");
            feed(&mut observer, &mut counter, true, 200, 102, 0x12, b"");
            let flow = observer.flows.values().next().expect("client SYN retained");
            assert!(flow.reassembly.is_none());
            assert!(flow.server_start.is_none());

            feed(&mut observer, &mut counter, reply, 201, 101, 4, b"");
            assert!(observer.flows.is_empty());
            assert!(receiver.try_recv().is_err());
        }
    }

    #[test]
    fn midstream_payload_without_a_witnessed_syn_is_ignored() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, receiver) = uploads(8);
        let mut observer = Observer::new(&artifact, sender);
        let mut counter = 0;
        let mut sequence = 201;
        for frame in test_server_frames(1) {
            feed(&mut observer, &mut counter, true, sequence, 101, 0x10, &frame);
            sequence += u32::try_from(frame.len()).expect("frame length");
        }
        assert!(observer.flows.is_empty());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn flow_capacity_evicts_the_least_recently_touched_decoder() {
        let artifact = RuntimeArtifact::test_fixture();
        let (_directory, _spool, sender, _receiver) = uploads(8);
        let mut observer = Observer::new(&artifact, sender);
        for index in 0..MAX_FLOWS {
            let mut bytes = packet(false, 100, 0, 2, b"");
            let port = u16::try_from(index + 1024).expect("unique client port");
            bytes[20..22].copy_from_slice(&port.to_be_bytes());
            observer.record(capture::Record {
                sequence: u32::try_from(index).expect("capture sequence"),
                reply: false,
                payload: &bytes,
            });
        }
        let oldest = (
            "192.0.2.10:1025".parse().expect("client"),
            "198.51.100.20:3101".parse().expect("server"),
        );
        observer.flows.get_mut(&oldest).expect("second flow").touched = Instant::now()
            .checked_sub(std::time::Duration::from_secs(60))
            .expect("earlier timestamp");
        let bytes = packet(false, 100, 0, 2, b"");
        observer.record(capture::Record {
            sequence: u32::try_from(MAX_FLOWS).expect("sequence"),
            reply: false,
            payload: &bytes,
        });

        assert_eq!(observer.flows.len(), MAX_FLOWS);
        assert!(!observer.flows.contains_key(&oldest));
    }

    #[test]
    fn entry_count_batching_preserves_every_payload_and_its_context() {
        let (_directory, _spool, writer, receiver) = uploads(8);
        let context = MailContext { player_id: Some(42), server_id: Some(10) };
        let entries: Vec<_> = (0_u32..513).map(|value| value.to_be_bytes().to_vec()).collect();
        submit_entries(&writer, &context, entries.clone());
        let first = receiver.try_recv().expect("full batch").into_batch();
        let second = receiver.try_recv().expect("remaining entry").into_batch();

        assert_eq!((first.entries.len(), second.entries.len()), (512, 1));
        assert_eq!(first.context, context);
        assert_eq!(second.context, context);
        let actual: Vec<_> =
            first.entries.into_iter().chain(second.entries).map(|bytes| bytes.to_vec()).collect();
        assert_eq!(actual, entries);
    }

    #[test]
    fn payload_limit_splits_batches_and_oversized_entries_do_not_hide_later_mail() {
        let (_directory, _spool, writer, receiver) = uploads(8);
        let context = MailContext::default();
        submit_entries(
            &writer,
            &context,
            vec![vec![1; MAX_BATCH_BYTES], vec![2; MAX_BATCH_BYTES + 1], b"tail".to_vec()],
        );

        let first = receiver.try_recv().expect("exact payload limit").into_batch();
        let second = receiver.try_recv().expect("entry following oversized mail").into_batch();
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first.entries[0].len(), MAX_BATCH_BYTES);
        assert_eq!(second.entries, vec![Bytes::from_static(b"tail")]);
        assert!(receiver.try_recv().is_err());
    }
}
