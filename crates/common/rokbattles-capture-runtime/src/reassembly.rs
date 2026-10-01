//! Bounded TCP ordering without inventing bytes across a capture gap.

use std::collections::BTreeMap;

pub const MAX_PENDING_BYTES: usize = 1024 * 1024;
const MAX_PENDING_SEGMENTS: usize = 256;

/// Sequence-relative reassembly for one server direction.
pub struct Reassembly {
    next: u32,
    pending: BTreeMap<u32, Vec<u8>>,
    buffered: usize,
}

impl std::fmt::Debug for Reassembly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reassembly")
            .field("pending_segments", &self.pending.len())
            .field("buffered_bytes", &self.buffered)
            .finish_non_exhaustive()
    }
}

impl Reassembly {
    pub fn new(next: u32) -> Self {
        Self { next, pending: BTreeMap::new(), buffered: 0 }
    }

    /// Next server sequence required for contiguous output.
    pub fn next_sequence(&self) -> u32 {
        self.next
    }

    /// Whether pending data would extend beyond a witnessed terminal sequence.
    pub fn buffered_beyond(&self, terminal: u32) -> bool {
        let limit = terminal.wrapping_sub(self.next);
        if limit >= 1 << 31 {
            return true;
        }

        self.pending.iter().any(|(sequence, bytes)| {
            u64::from(sequence.wrapping_sub(self.next)) + bytes.len() as u64 > u64::from(limit)
        })
    }

    pub fn buffered(&self) -> usize {
        self.buffered
    }

    /// Return newly contiguous bytes, ignoring already consumed retransmissions.
    ///
    /// # Errors
    /// Rejects ambiguous pending overlaps and excessive reordering. The caller
    /// must retire observation on error; forwarding has already happened.
    pub fn push(&mut self, sequence: u32, mut payload: &[u8]) -> Result<Vec<u8>, &'static str> {
        if payload.len() > MAX_PENDING_BYTES {
            return Err("TCP segment exceeds reorder budget");
        }
        let distance = sequence.wrapping_sub(self.next);
        if distance == 1 << 31 {
            return Err("ambiguous TCP sequence distance");
        }
        let offset = distance as i32;
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
        // Pending overlaps must agree, regardless of segmentation/retransmission.
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
            if self.buffered - previous + payload.len() > MAX_PENDING_BYTES
                || self.pending.len() >= MAX_PENDING_SEGMENTS
            {
                return Err("TCP reorder buffer full");
            }
            self.buffered = self.buffered - previous + payload.len();
            self.pending.insert(start, payload.to_vec());
            return Ok(Vec::new());
        }
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
    fn exact_half_range_is_ambiguous_and_oversized_segments_are_rejected() {
        let mut stream = Reassembly::new(u32::MAX);
        assert_eq!(
            stream.push(u32::MAX.wrapping_add(1 << 31), b"x"),
            Err("ambiguous TCP sequence distance")
        );
        assert_eq!(
            stream.push(u32::MAX, &vec![0; MAX_PENDING_BYTES + 1]),
            Err("TCP segment exceeds reorder budget")
        );
        assert_eq!(stream.buffered(), 0);
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
}
