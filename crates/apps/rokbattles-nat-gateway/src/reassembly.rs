//! Restore contiguous server bytes from bounded, reordered TCP segments.
//!
//! A stream cipher must see each byte exactly once and in order. [`Reassembly`]
//! buffers gaps, verifies overlapping pending bytes, and trims retransmissions
//! that precede the next expected sequence number. It never fills a gap with
//! invented bytes or tries to repair an ambiguous overlap.
//!
//! Sequence comparisons use signed wrapping differences. They assume segments
//! belong to the same connection and are less than half the TCP sequence space
//! apart; the accepted forward gap is further restricted to 1 MiB.

use std::collections::BTreeMap;

const MAX_PENDING_BYTES: usize = 1024 * 1024;
const MAX_PENDING_SEGMENTS: usize = 256;

/// Pending bytes for one TCP direction, starting at a known sequence number.
///
/// At most 1 MiB across 256 pending segments is retained. Consumed bytes are not
/// stored, so retransmitted prefixes cannot be compared with past output.
///
/// # Examples
///
/// ```
/// use rokbattles_nat_gateway::reassembly::Reassembly;
///
/// let mut stream = Reassembly::new(100);
/// assert!(stream.push(103, b"def")?.is_empty());
/// assert_eq!(stream.push(100, b"abc")?, b"abcdef");
/// assert_eq!(stream.buffered(), 0);
/// # Ok::<(), &'static str>(())
/// ```
#[derive(Debug)]
pub struct Reassembly {
    next: u32,
    pending: BTreeMap<u32, Vec<u8>>,
    buffered: usize,
}

impl Reassembly {
    /// Start at `next`, normally the server SYN/ACK sequence number plus one.
    pub fn new(next: u32) -> Self {
        Self { next, pending: BTreeMap::new(), buffered: 0 }
    }

    /// Return bytes retained in pending segments, counting agreeing overlaps.
    pub fn buffered(&self) -> usize {
        self.buffered
    }

    /// Insert payload at `sequence` and return all newly contiguous bytes.
    ///
    /// `sequence` names the first payload byte; the caller must account for SYN.
    /// A gap returns an empty vector. Fully consumed retransmissions are ignored;
    /// a retransmission extending beyond consumed output contributes only its
    /// new tail. Pending overlaps must contain identical bytes.
    ///
    /// # Errors
    ///
    /// Returns a reason when pending bytes conflict, a forward gap exceeds 1 MiB,
    /// or the byte/segment budget would be exceeded. The observer must discard
    /// the connection's reassembly and cipher state after an error; this result
    /// does not affect kernel forwarding.
    pub fn push(&mut self, sequence: u32, mut payload: &[u8]) -> Result<Vec<u8>, &'static str> {
        // Signed modular distance treats bytes just before a wrap as consumed,
        // rather than as an enormous forward gap. See the module's range assumption.
        let offset = sequence.wrapping_sub(self.next) as i32;
        if offset < 0 {
            let skip = offset.unsigned_abs() as usize;
            if skip >= payload.len() {
                return Ok(Vec::new());
            }
            payload = payload.get(skip..).ok_or("invalid overlap")?;
        }
        if payload.is_empty() {
            return Ok(Vec::new());
        }
        let start = if offset < 0 { self.next } else { sequence };
        if offset > MAX_PENDING_BYTES as i32 {
            return Err("TCP gap exceeds reorder window");
        }

        // Only pending bytes can be compared: consumed output is no longer retained.
        // Do this before insertion so conflicting retransmissions cannot replace it.
        let start_offset = start.wrapping_sub(self.next) as usize;
        for (existing, bytes) in &self.pending {
            let old = existing.wrapping_sub(self.next) as usize;
            let lo = old.max(start_offset);
            let hi = (old + bytes.len()).min(start_offset + payload.len());
            if lo < hi
                && bytes.get(lo - old..hi - old)
                    != payload.get(lo - start_offset..hi - start_offset)
            {
                return Err("conflicting TCP overlap");
            }
        }

        if start != self.next {
            if self.pending.get(&start).is_some_and(|old| old.len() >= payload.len()) {
                return Ok(Vec::new());
            }
            let previous = self.pending.get(&start).map_or(0, Vec::len);
            // A longer agreeing retransmission replaces its old allocation. It
            // consumes additional bytes, but it does not consume another slot.
            if self.buffered - previous + payload.len() > MAX_PENDING_BYTES
                || (previous == 0 && self.pending.len() >= MAX_PENDING_SEGMENTS)
            {
                return Err("TCP reorder buffer full");
            }
            self.buffered = self.buffered - previous + payload.len();
            self.pending.insert(start, payload.to_vec());
            return Ok(Vec::new());
        }

        // Closing a gap may expose several overlapping pending segments. Their
        // integer key order is not TCP order when sequence numbers wrap.
        let mut output = payload.to_vec();
        self.next = self.next.wrapping_add(payload.len() as u32);
        loop {
            let candidate =
                self.pending.keys().copied().find(|seq| seq.wrapping_sub(self.next) as i32 <= 0);
            let Some(sequence) = candidate else {
                break;
            };
            let Some(bytes) = self.pending.remove(&sequence) else {
                break;
            };
            self.buffered -= bytes.len();
            let consumed = self.next.wrapping_sub(sequence) as usize;
            if let Some(tail) = bytes.get(consumed..) {
                output.extend_from_slice(tail);
                self.next = self.next.wrapping_add(tail.len() as u32);
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_retransmit_and_sequence_wrap_preserve_bytes() {
        let mut stream = Reassembly::new(u32::MAX - 2);
        assert_eq!(stream.push(0, b"def").expect("pending"), b"");
        assert_eq!(stream.push(u32::MAX - 2, b"abc").expect("ordered"), b"abcdef");
        assert_eq!(stream.push(0, b"def").expect("duplicate"), b"");
        assert_eq!(stream.push(3, b"ghi").expect("next"), b"ghi");
    }

    #[test]
    fn conflicting_pending_overlap_is_rejected() {
        let mut stream = Reassembly::new(10);
        stream.push(12, b"ab").expect("pending");
        assert_eq!(stream.push(11, b"xyz"), Err("conflicting TCP overlap"));
    }

    #[test]
    fn agreeing_partial_overlaps_emit_each_byte_once() {
        let mut stream = Reassembly::new(100);
        stream.push(103, b"def").expect("pending tail");
        stream.push(102, b"cde").expect("agreeing overlap");

        assert_eq!(stream.push(100, b"abc").expect("close gap"), b"abcdef");
        assert_eq!(stream.buffered(), 0);
    }

    #[test]
    fn consumed_retransmissions_cannot_replace_output() {
        let mut stream = Reassembly::new(10);
        assert_eq!(stream.push(10, b"trusted").expect("ordered"), b"trusted");

        assert_eq!(stream.push(10, b"hostile").expect("old retransmission"), b"");
        assert_eq!(stream.push(17, b" next").expect("continued stream"), b" next");
    }

    #[test]
    fn gap_and_segment_budgets_fail_closed() {
        let mut stream = Reassembly::new(0);
        assert_eq!(
            stream.push(u32::try_from(MAX_PENDING_BYTES + 1).expect("limit"), b"x"),
            Err("TCP gap exceeds reorder window")
        );

        for sequence in 1..=MAX_PENDING_SEGMENTS {
            stream
                .push(u32::try_from(sequence * 2).expect("test sequence"), b"x")
                .expect("segment should fit");
        }
        assert_eq!(
            stream.push(u32::try_from(MAX_PENDING_SEGMENTS * 2 + 2).expect("sequence"), b"x"),
            Err("TCP reorder buffer full")
        );
    }

    #[test]
    fn pending_byte_budget_accounts_for_replacement() {
        let mut stream = Reassembly::new(0);
        stream.push(1, &[1; 32]).expect("initial pending bytes");
        stream.push(1, &[1; 64]).expect("larger agreeing replacement");

        assert_eq!(stream.buffered(), 64);
        assert_eq!(stream.push(0, b"z").expect("close gap").len(), 65);
        assert_eq!(stream.buffered(), 0);
    }

    #[test]
    fn replacing_a_pending_segment_does_not_require_another_slot() {
        let mut stream = Reassembly::new(0);
        for index in 0..MAX_PENDING_SEGMENTS {
            let sequence = u32::try_from(index * 4 + 4).expect("sequence");
            stream.push(sequence, b"a").expect("slot available");
        }

        stream.push(4, b"ab").expect("replacement uses its existing slot");
        assert_eq!(stream.buffered(), MAX_PENDING_SEGMENTS + 1);
        assert_eq!(stream.push(0, b"0123").expect("close first gap"), b"0123ab");
    }

    #[test]
    fn segment_permutations_and_duplicates_preserve_output_across_wrap() {
        let orders = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
        for origin in [0_u32, 100, u32::MAX - 3] {
            for order in orders {
                let mut stream = Reassembly::new(origin);
                let mut output = Vec::new();
                for index in order {
                    let start = index * 3;
                    let sequence = origin.wrapping_add(u32::try_from(start).expect("offset"));
                    let payload = &b"abcdefghi"[start..start + 3];
                    output.extend(stream.push(sequence, payload).expect("segment"));
                    assert!(stream.push(sequence, payload).expect("duplicate").is_empty());
                }
                assert_eq!(output, b"abcdefghi", "origin={origin}, order={order:?}");
                assert_eq!(stream.buffered(), 0);
            }
        }
    }

    #[test]
    fn retransmission_contributes_only_its_unconsumed_tail() {
        let mut stream = Reassembly::new(10);
        assert_eq!(stream.push(10, b"abc").expect("initial bytes"), b"abc");
        assert_eq!(stream.push(11, b"bcdef").expect("overlapping tail"), b"def");
        assert_eq!(stream.push(16, b"g").expect("next byte"), b"g");
    }

    #[test]
    fn exact_pending_byte_limit_is_accepted_and_excess_does_not_replace_it() {
        let mut stream = Reassembly::new(0);
        stream.push(1, &vec![1; MAX_PENDING_BYTES]).expect("exact byte limit");
        assert_eq!(stream.push(1, &vec![1; MAX_PENDING_BYTES + 1]), Err("TCP reorder buffer full"));
        assert_eq!(stream.buffered(), MAX_PENDING_BYTES);
        let output = stream.push(0, b"x").expect("close gap");
        assert_eq!(output.len(), MAX_PENDING_BYTES + 1);
        assert_eq!(stream.buffered(), 0);
    }
}
