//! Stateful server-stream framing and decryption.

use zeroize::Zeroize;

use crate::{
    RuntimeArtifact,
    artifact::LOGIN_API_ID,
    protobuf::{ProtocolError, mail_candidates, parse_handshake, parse_login, visit_messages},
};

/// Maximum body length accepted from a short or extended frame prefix.
pub const MAX_FRAME_BODY_BYTES: usize = 25 * 1024 * 1024;

/// Metadata or raw mail entries extracted from a complete decrypted frame.
#[derive(PartialEq, Eq)]
pub enum StreamEvent {
    /// Login context used for later mail reconstruction.
    Login {
        /// Player reported by the server.
        player_id: i64,
        /// Kingdom reported by the server.
        server_id: i32,
    },
    /// Raw MailEntity protobuf entries, before ingress reconstruction.
    Mails {
        /// Optional server ID learned from an entry.
        server_id: Option<i32>,
        /// Exact protobuf entry bytes.
        entries: Vec<Vec<u8>>,
        /// Remaining initial-sync pages, when the carrier supplies this field.
        remaining: Option<usize>,
    },
}

impl std::fmt::Debug for StreamEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Login { .. } => f.write_str("Login { .. }"),
            Self::Mails { entries, .. } => f
                .debug_struct("Mails")
                .field("entries", &entries.len())
                .field("bytes", &entries.iter().map(Vec::len).sum::<usize>())
                .finish_non_exhaustive(),
        }
    }
}

/// A failure after which callers must retire this decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    #[error("frame body exceeded the configured limit")]
    /// A frame prefix declares more than the accepted bound.
    FrameTooLarge,
    #[error("stream decode work budget exhausted")]
    /// Shared frame/inflation work budget was exhausted.
    WorkBudgetExceeded,
    #[error("stream memory budget exhausted")]
    /// Caller could not admit additional incomplete-frame memory.
    MemoryBudgetExceeded,
    #[error("stream decoder has been retired")]
    /// An earlier error permanently retired this decoder.
    Retired,
    #[error("first server frame was not a supported handshake")]
    /// The first frame cannot initialize the supported cipher.
    UnsupportedHandshake,
    #[error("encrypted frame arrived before cipher initialization")]
    /// Encrypted data arrived without an initialized cipher.
    CipherUnavailable,
    #[error("compressed protocol payload was incompatible")]
    /// A compressed envelope is malformed or exceeds its inflation bound.
    Decompression,
    #[error("decrypted protocol data was incompatible")]
    /// Decrypted data does not match the artifact wire schema.
    Protocol,
}

impl From<ProtocolError> for StreamError {
    fn from(value: ProtocolError) -> Self {
        match value {
            ProtocolError::WorkBudgetExceeded => Self::WorkBudgetExceeded,
            ProtocolError::DeclaredInflationTooLarge
            | ProtocolError::InflationTooLarge
            | ProtocolError::InflationLengthMismatch
            | ProtocolError::Inflate => Self::Decompression,
            _ => Self::Protocol,
        }
    }
}

/// Stateful decoder for one contiguous server stream beginning at its handshake.
///
/// This type does not reorder TCP, buffer capture gaps or write mail. Reuse it
/// for the entire connection; creating a replacement midstream loses cipher
/// position. Incomplete frame bodies are bounded and retained between pushes.
pub struct ServerStreamProcessor<'a> {
    artifact: &'a RuntimeArtifact,
    failed: bool,
    first_frame_limit: usize,
    frame_limit: usize,
    prefix: [u8; 2],
    prefix_len: usize,
    extended: [u8; 4],
    extended_len: usize,
    remaining: Option<usize>,
    body: Vec<u8>,
    frame_index: u64,
    cipher: Option<StreamCipher>,
}

impl<'a> ServerStreamProcessor<'a> {
    /// Begin decoding a new server stream using a validated artifact.
    pub fn new(artifact: &'a RuntimeArtifact) -> Self {
        Self {
            artifact,
            failed: false,
            first_frame_limit: MAX_FRAME_BODY_BYTES,
            frame_limit: MAX_FRAME_BODY_BYTES,
            prefix: [0; 2],
            prefix_len: 0,
            extended: [0; 4],
            extended_len: 0,
            remaining: None,
            body: Vec::new(),
            frame_index: 0,
            cipher: None,
        }
    }

    /// Reduce frame limits for untrusted intake; never enlarge the protocol maximum.
    pub fn with_frame_limits(mut self, first_frame: usize, subsequent_frames: usize) -> Self {
        self.first_frame_limit = first_frame.min(MAX_FRAME_BODY_BYTES);
        self.frame_limit = subsequent_frames.min(MAX_FRAME_BODY_BYTES);
        self
    }

    /// Bytes reserved for an incomplete frame.
    pub fn buffered_bytes(&self) -> usize {
        self.body.capacity()
    }

    /// Complete protocol frames consumed, for a caller's frame-assembly deadline.
    pub fn completed_frames(&self) -> u64 {
        self.frame_index
    }

    /// Whether a prefix or frame body is awaiting additional bytes.
    pub fn has_incomplete_frame(&self) -> bool {
        self.prefix_len != 0 || self.extended_len != 0 || self.remaining.is_some()
    }

    /// Consume the next contiguous server bytes and return complete events.
    ///
    /// # Errors
    /// Returns an error for an unsupported handshake, invalid frame length,
    /// malformed protocol message or unsafe compressed payload. State may have
    /// advanced when an error is returned; callers must retire this instance.
    pub fn push(&mut self, payload: &[u8]) -> Result<Vec<StreamEvent>, StreamError> {
        let mut work_remaining = usize::MAX;
        self.push_with_budget(payload, &mut work_remaining)
    }

    /// Decode with a caller-owned work budget shared across frames and inflations.
    /// Charges declared inflation before allocation; no per-frame budget reset.
    ///
    /// # Errors
    /// Returns a protocol error or [`StreamError::WorkBudgetExceeded`]. The caller
    /// must retire this decoder after any error, never retry bytes against it.
    pub fn push_with_budget(
        &mut self,
        payload: &[u8],
        work_remaining: &mut usize,
    ) -> Result<Vec<StreamEvent>, StreamError> {
        self.push_bounded(payload, work_remaining, usize::MAX)
    }

    /// Decode while also limiting this decoder's incomplete-frame allocation.
    ///
    /// # Errors
    /// Returns the same errors as [`Self::push_with_budget`] or a memory limit
    /// error before growing a frame beyond `maximum_buffered`.
    pub fn push_bounded(
        &mut self,
        payload: &[u8],
        work_remaining: &mut usize,
        maximum_buffered: usize,
    ) -> Result<Vec<StreamEvent>, StreamError> {
        if self.failed {
            return Err(StreamError::Retired);
        }
        let result = self.push_inner(payload, work_remaining, maximum_buffered);
        if result.is_err() {
            self.retire();
        }
        result
    }

    fn push_inner(
        &mut self,
        payload: &[u8],
        work_remaining: &mut usize,
        maximum_buffered: usize,
    ) -> Result<Vec<StreamEvent>, StreamError> {
        *work_remaining =
            work_remaining.checked_sub(payload.len()).ok_or(StreamError::WorkBudgetExceeded)?;
        let mut events = WipedEvents(Vec::new());
        let mut position = 0usize;
        while position < payload.len() {
            if self.remaining.is_none() {
                self.read_prefix(payload, &mut position)?;
                let Some(remaining) = self.remaining else {
                    return Ok(events.finish());
                };
                if remaining == 0 {
                    events.extend(self.complete_frame(work_remaining)?);
                    continue;
                }
            }

            let remaining = self.remaining.ok_or(StreamError::FrameTooLarge)?;
            let available = payload.len().saturating_sub(position);
            let take = remaining.min(available);
            let end = position.checked_add(take).ok_or(StreamError::FrameTooLarge)?;
            let encrypted = payload.get(position..end).ok_or(StreamError::FrameTooLarge)?;
            let body_start = self.body.len();
            let required = body_start.checked_add(take).ok_or(StreamError::MemoryBudgetExceeded)?;
            if required > maximum_buffered {
                return Err(StreamError::MemoryBudgetExceeded);
            }
            if required > self.body.capacity() {
                // Geometric growth keeps one-byte wire chunks amortized-linear;
                // an exact reserve on every byte would repeatedly copy the frame.
                let frame_limit =
                    if self.frame_index == 0 { self.first_frame_limit } else { self.frame_limit };
                let capacity = required
                    .checked_next_power_of_two()
                    .ok_or(StreamError::MemoryBudgetExceeded)?
                    .min(maximum_buffered)
                    .min(frame_limit);
                self.body
                    .try_reserve_exact(capacity.saturating_sub(body_start))
                    .map_err(|_error| StreamError::MemoryBudgetExceeded)?;
                if self.body.capacity() > maximum_buffered {
                    return Err(StreamError::MemoryBudgetExceeded);
                }
            }
            self.body.extend_from_slice(encrypted);
            if self.frame_index > 0 {
                let cipher = self.cipher.as_mut().ok_or(StreamError::CipherUnavailable)?;
                let appended = self.body.get_mut(body_start..).ok_or(StreamError::FrameTooLarge)?;
                cipher.apply(appended);
            }
            position = end;
            let next_remaining = remaining.saturating_sub(take);
            self.remaining = Some(next_remaining);
            if next_remaining == 0 {
                events.extend(self.complete_frame(work_remaining)?);
            }
        }
        Ok(events.finish())
    }

    fn read_prefix(&mut self, payload: &[u8], position: &mut usize) -> Result<(), StreamError> {
        take_into(&mut self.prefix, &mut self.prefix_len, payload, position);
        if self.prefix_len < self.prefix.len() {
            return Ok(());
        }
        let short = u16::from_be_bytes(self.prefix);
        let length = if short == u16::MAX {
            take_into(&mut self.extended, &mut self.extended_len, payload, position);
            if self.extended_len < self.extended.len() {
                return Ok(());
            }
            usize::try_from(u32::from_be_bytes(self.extended))
                .map_err(|_error| StreamError::FrameTooLarge)?
        } else {
            usize::from(short)
        };
        let limit = if self.frame_index == 0 { self.first_frame_limit } else { self.frame_limit };
        if length > limit {
            return Err(StreamError::FrameTooLarge);
        }

        self.prefix_len = 0;
        self.extended_len = 0;
        self.body.zeroize();
        self.body.clear();
        // Grow with received bytes; an untrusted length must not reserve 25 MiB.
        self.remaining = Some(length);
        Ok(())
    }

    fn complete_frame(
        &mut self,
        work_remaining: &mut usize,
    ) -> Result<Vec<StreamEvent>, StreamError> {
        self.remaining = None;
        let mut events = WipedEvents(Vec::new());
        if self.frame_index == 0 {
            let (key1, _key2) = parse_handshake(&self.body, self.artifact)
                .map_err(|_error| StreamError::UnsupportedHandshake)?;
            self.cipher = Some(StreamCipher::new(server_secret(key1)));
        } else {
            let mut entry_budget = crate::protobuf::MAX_MAIL_ENTRIES;
            visit_messages(&self.body, self.artifact, work_remaining, |message| {
                if message.api_id == LOGIN_API_ID {
                    let (player_id, server_id) = parse_login(&message.payload, self.artifact)?;
                    events.push(StreamEvent::Login { player_id, server_id });
                } else if self.artifact.carriers.contains_key(&message.api_id) {
                    let candidates = mail_candidates(
                        &message.payload,
                        self.artifact,
                        message.api_id,
                        entry_budget,
                    )?;
                    entry_budget -= candidates.entries.len();
                    if !candidates.entries.is_empty() || candidates.remaining.is_some() {
                        events.push(StreamEvent::Mails {
                            server_id: candidates.server_id,
                            entries: candidates.entries,
                            remaining: candidates.remaining,
                        });
                    }
                }
                Ok(())
            })?;
        }
        self.frame_index = self.frame_index.saturating_add(1);
        self.body.zeroize();
        self.body.clear();
        Ok(events.finish())
    }
}

impl ServerStreamProcessor<'_> {
    fn retire(&mut self) {
        self.failed = true;
        self.body.zeroize();
        self.body = Vec::new();
        self.prefix.zeroize();
        self.extended.zeroize();
        self.prefix_len = 0;
        self.extended_len = 0;
        self.remaining = None;
        if let Some(mut cipher) = self.cipher.take() {
            cipher.words.zeroize();
        }
    }
}

struct WipedEvents(Vec<StreamEvent>);

impl WipedEvents {
    fn finish(mut self) -> Vec<StreamEvent> {
        std::mem::take(&mut self.0)
    }
}

impl std::ops::Deref for WipedEvents {
    type Target = Vec<StreamEvent>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for WipedEvents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for WipedEvents {
    fn drop(&mut self) {
        for event in &mut self.0 {
            if let StreamEvent::Mails { entries, .. } = event {
                for entry in entries {
                    entry.zeroize();
                }
            }
        }
    }
}

impl std::fmt::Debug for ServerStreamProcessor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerStreamProcessor")
            .field("buffered_bytes", &self.body.len())
            .field("frames", &self.frame_index)
            .finish_non_exhaustive()
    }
}

impl Drop for ServerStreamProcessor<'_> {
    fn drop(&mut self) {
        self.retire();
    }
}

fn take_into<const N: usize>(
    target: &mut [u8; N],
    target_len: &mut usize,
    payload: &[u8],
    position: &mut usize,
) {
    let needed = N.saturating_sub(*target_len);
    let available = payload.len().saturating_sub(*position);
    let take = needed.min(available);
    let Some(source) = payload.get(*position..position.saturating_add(take)) else {
        return;
    };
    let Some(destination) = target.get_mut(*target_len..target_len.saturating_add(take)) else {
        return;
    };
    destination.copy_from_slice(source);
    *target_len = target_len.saturating_add(take);
    *position = position.saturating_add(take);
}

fn server_secret(value: u64) -> u32 {
    let secret = (value >> 1).saturating_add(0x400);
    (secret & u64::from(u32::MAX)) as u32
}

struct StreamCipher {
    words: Vec<u32>,
}

impl StreamCipher {
    fn new(seed: u32) -> Self {
        let mut stream = Self { words: vec![0; 0x1400 / 4] };
        stream.seed(seed);
        stream
    }

    fn get(&self, offset: usize) -> u32 {
        self.words.get(offset / 4).copied().unwrap_or_default()
    }

    fn set(&mut self, offset: usize, value: u32) {
        if let Some(slot) = self.words.get_mut(offset / 4) {
            *slot = value;
        }
    }

    fn array_get(&self, base: usize, index: u32) -> u32 {
        let index = usize::try_from(index & 0x3f).unwrap_or_default();
        self.get(base.saturating_add(4usize.saturating_mul(index)))
    }

    fn array_set(&mut self, base: usize, index: u32, value: u32) {
        let index = usize::try_from(index & 0x3f).unwrap_or_default();
        self.set(base.saturating_add(4usize.saturating_mul(index)), value);
    }

    fn seed(&mut self, seed: u32) {
        let x = seed ^ 0x0502_7919;
        self.set(0x00, x);
        self.set(0x0c, 0x37);
        self.set(0x10, 0x18);
        self.set(0x120, 0x39);
        self.set(0x124, 0x07);
        self.set(0x234, 0x3a);
        self.set(0x238, 0x13);
        self.set(0x08, (x >> 1) | (Self::seed_feedback(x) & 0x8000_0000));

        let ecx = (((x << 1) ^ (x >> 1)) & 0x5555_5555) ^ (x << 1);
        self.set(0x11c, Self::twist(ecx));
        let ecx2 = !((((x << 4) ^ (x >> 4)) & 0x0f0f_0f0f) ^ (x << 4));
        self.set(0x230, Self::twist(ecx2));

        for (base, source_offset) in [(0x1c, 0x08), (0x130, 0x11c), (0x244, 0x230)] {
            let mut value = self.get(source_offset);
            for index in 0..64 {
                for _ in 0..16 {
                    value = Self::twist(value);
                    value = Self::twist(value);
                }
                self.array_set(base, index, value);
            }
            self.set(base.saturating_sub(8), 0x3f);
            self.set(base.saturating_sub(4), 0);
        }
        self.set(0x04, 0x1000);
    }

    const fn twist(value: u32) -> u32 {
        let mut ecx = (value << 1) ^ value;
        ecx = (ecx << 1) ^ value;
        ecx = (ecx << 2) ^ value;
        ecx = (ecx << 2) ^ value;
        ecx = (ecx << 25) ^ value;
        (ecx & 0x8000_0000) | (value >> 1)
    }

    const fn seed_feedback(value: u32) -> u32 {
        let mut eax = (value << 1) ^ value;
        eax = (eax << 1) ^ value;
        eax = (eax << 2) ^ value;
        eax = (eax << 2) ^ value;
        (eax << 25) ^ value
    }

    fn update_group(
        &mut self,
        index_offset: usize,
        carry_offset: usize,
        lag_a_offset: usize,
        lag_b_offset: usize,
        base: usize,
    ) {
        let index = self.get(index_offset).wrapping_add(1) & 0x3f;
        self.set(index_offset, index);
        let a_index = index.wrapping_sub(self.get(lag_b_offset)) & 0x3f;
        let b_index = index.wrapping_sub(self.get(lag_a_offset)) & 0x3f;
        let a = self.array_get(base, a_index);
        let b = self.array_get(base, b_index);
        let value = a.wrapping_add(b);
        self.array_set(base, index, value);
        self.set(carry_offset, u32::from(value < a || value < b));
    }

    fn refill(&mut self) {
        for output_index in 0..0x400 {
            let carry_sum =
                self.get(0x12c).wrapping_add(self.get(0x18)).wrapping_add(self.get(0x240));
            if carry_sum == 0 || carry_sum == 3 {
                self.update_group(0x14, 0x18, 0x0c, 0x10, 0x1c);
                self.update_group(0x128, 0x12c, 0x120, 0x124, 0x130);
                self.update_group(0x23c, 0x240, 0x234, 0x238, 0x244);
            } else {
                let required = u32::from(carry_sum == 2);
                if self.get(0x18) == required {
                    self.update_group(0x14, 0x18, 0x0c, 0x10, 0x1c);
                }
                if self.get(0x12c) == required {
                    self.update_group(0x128, 0x12c, 0x120, 0x124, 0x130);
                }
                if self.get(0x240) == required {
                    self.update_group(0x23c, 0x240, 0x234, 0x238, 0x244);
                }
            }

            let value = self.array_get(0x244, self.get(0x23c))
                ^ self.array_get(0x130, self.get(0x128))
                ^ self.array_get(0x1c, self.get(0x14));
            self.set(0x344 + 4 * output_index, value);
        }
        self.set(0x04, 0);
    }

    fn apply(&mut self, data: &mut [u8]) {
        let mut position = 0usize;
        while position < data.len() {
            let mut offset = self.get(0x04) as usize;
            if offset >= 0x1000 {
                self.refill();
                offset = 0;
            }
            let take =
                (0x1000usize.saturating_sub(offset)).min(data.len().saturating_sub(position));
            let Some(chunk) = data.get_mut(position..position.saturating_add(take)) else {
                return;
            };
            for (index, byte) in chunk.iter_mut().enumerate() {
                let stream_offset = offset.saturating_add(index);
                let word = self.get(0x344 + 4 * (stream_offset / 4)).to_le_bytes();
                *byte ^= word.get(stream_offset % 4).copied().unwrap_or_default();
            }
            offset = offset.saturating_add(take);
            self.set(0x04, u32::try_from(offset).unwrap_or(u32::MAX));
            position = position.saturating_add(take);
        }
    }
}

/// Synthetic handshake, login and encrypted mail frames for transport tests.
///
/// Generates all bytes in memory without reading a protocol artifact.
pub fn test_server_frames(mail_count: usize) -> Vec<Vec<u8>> {
    let handshake = vec![
        0, 17, 0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8, 0xb3,
        0x98, 0xf1, 0x03,
    ];
    let mut frames = vec![handshake];
    let mut cipher = StreamCipher::new(server_secret(626_273_431));
    let mut login = vec![8, 15, 18, 5, 8, 42, 16, 0x8c, 0x0e];
    cipher.apply(&mut login);
    let mut framed = (login.len() as u16).to_be_bytes().to_vec();
    framed.extend(login);
    frames.push(framed);
    for _ in 0..mail_count {
        let mut body = vec![8, 0xe5, 0x3d, 18, 19, 10, 17, 10, 7];
        body.extend(b"mail-id");
        body.extend([74, 6]);
        body.extend(b"Battle");
        cipher.apply(&mut body);
        let mut framed = (body.len() as u16).to_be_bytes().to_vec();
        framed.extend(body);
        frames.push(framed);
    }
    frames
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::ZlibEncoder};

    use super::*;

    const HANDSHAKE_BODY: &[u8] = &[
        0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8, 0xb3, 0x98,
        0xf1, 0x03,
    ];

    #[test]
    fn stream_cipher_matches_preserved_server_fixture() {
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let mut encrypted = vec![0xef, 0xad, 0x88, 0xc8];

        cipher.apply(&mut encrypted);

        assert_eq!(encrypted, [0x08, 0x36, 0x12, 0x00]);
    }

    #[test]
    fn processor_handles_handshake_split_across_reads() {
        let artifact = RuntimeArtifact::test_fixture();
        let frame = frame(HANDSHAKE_BODY);
        let mut processor = ServerStreamProcessor::new(&artifact);

        processor.push(&frame[..1]).expect("prefix fragment should parse");
        processor.push(&frame[1..8]).expect("body fragment should parse");
        processor.push(&frame[8..]).expect("remaining fragment should parse");

        assert_eq!(processor.frame_index, 1);
    }

    #[test]
    fn processor_rejects_oversized_extended_frame() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        let length = u32::try_from(MAX_FRAME_BODY_BYTES + 1).expect("frame bound should fit u32");
        let mut payload = Vec::from(u16::MAX.to_be_bytes());
        payload.extend_from_slice(&length.to_be_bytes());

        let error = processor.push(&payload).expect_err("frame should be oversized");

        assert_eq!(error, StreamError::FrameTooLarge);
    }

    #[test]
    fn processor_handles_direct_and_both_compressed_mail_carriers() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let direct_7909 = msg(7909, &carrier_payload(1));
        let direct_7901 = msg(7901, &carrier_payload(1));
        let zmsg_7921 =
            msg(crate::artifact::ZMSG_API_ID, &compressed_payload(&msg(7921, &carrier_payload(2))));
        let report = bytes_field(1, &msg(7927, &carrier_payload(1)));
        let compressed_7927 = msg(crate::artifact::COMPRESSED_API_ID, &compressed_payload(&report));

        let mut stream = frame(HANDSHAKE_BODY);
        for body in [direct_7909, direct_7901, zmsg_7921, compressed_7927] {
            stream.extend(encrypted_frame(&mut cipher, &body));
        }
        let events =
            processor.push(&stream).expect("coalesced direct and wrapped frames should parse");

        assert_eq!(processor.frame_index, 5);
        assert_eq!(
            events
                .iter()
                .map(|event| match event {
                    StreamEvent::Mails { entries, .. } => entries.len(),
                    StreamEvent::Login { .. } => 0,
                })
                .sum::<usize>(),
            5
        );
    }

    #[test]
    fn compound_frame_preserves_login_and_five_distinct_reports_before_other_messages() {
        let artifact = RuntimeArtifact::test_fixture();
        let entries: Vec<_> = (0..5)
            .map(|index| {
                [bytes_field(1, format!("report-{index}").as_bytes()), bytes_field(9, b"Battle")]
                    .concat()
            })
            .collect();
        let mut compound = bytes_field(
            1,
            &msg(LOGIN_API_ID, &[varint_field(1, 42), varint_field(2, 1804)].concat()),
        );
        for entry in &entries {
            compound.extend(bytes_field(1, &msg(7909, &bytes_field(1, entry))));
        }
        compound.extend(bytes_field(1, &msg(54, &[])));
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let mut stream = frame(HANDSHAKE_BODY);
        stream.extend(encrypted_frame(
            &mut cipher,
            &msg(crate::artifact::COMPRESSED_API_ID, &compressed_payload(&compound)),
        ));
        for chunk_size in [1, 7, stream.len()] {
            let mut processor = ServerStreamProcessor::new(&artifact);
            let events: Vec<_> = stream
                .chunks(chunk_size)
                .flat_map(|chunk| processor.push(chunk).expect("compound should decode"))
                .collect();
            assert!(matches!(
                events.first(),
                Some(StreamEvent::Login { player_id: 42, server_id: 1804 })
            ));
            let observed: Vec<_> = events
                .into_iter()
                .filter_map(|event| match event {
                    StreamEvent::Mails { entries, .. } => Some(entries),
                    _ => None,
                })
                .flatten()
                .collect();
            assert_eq!(observed, entries);
        }
    }

    #[test]
    fn compound_rejects_excess_members_and_malformed_later_messages() {
        let artifact = RuntimeArtifact::test_fixture();
        let valid = bytes_field(1, &msg(7909, &carrier_payload(1)));
        for compound in [
            [valid.clone(), bytes_field(1, &[0])].concat(),
            [valid.clone(), varint_field(1, 0)].concat(),
            valid.repeat(crate::protobuf::MAX_COMPOUND_MESSAGES + 1),
        ] {
            let mut processor = ServerStreamProcessor::new(&artifact);
            let mut cipher = StreamCipher::new(server_secret(626_273_431));
            let mut stream = frame(HANDSHAKE_BODY);
            stream.extend(encrypted_frame(
                &mut cipher,
                &msg(crate::artifact::COMPRESSED_API_ID, &compressed_payload(&compound)),
            ));
            assert!(
                processor.push(&stream).is_err(),
                "invalid compound must not return partial mail events"
            );
        }
    }

    #[test]
    fn mail_entry_budget_applies_across_carriers_and_resets_each_frame() {
        let artifact = RuntimeArtifact::test_fixture();
        let half = bytes_field(1, &[]).repeat(crate::protobuf::MAX_MAIL_ENTRIES / 2);
        let member = bytes_field(1, &msg(7921, &half));
        let at_limit = member.repeat(2);
        for (compound, accepted) in [
            (at_limit.clone(), true),
            ([at_limit, bytes_field(1, &msg(7921, &bytes_field(1, &[])))].concat(), false),
            (
                bytes_field(
                    1,
                    &msg(7921, &bytes_field(1, &[]).repeat(crate::protobuf::MAX_MAIL_ENTRIES + 1)),
                ),
                false,
            ),
        ] {
            let mut processor = ServerStreamProcessor::new(&artifact);
            let mut cipher = StreamCipher::new(server_secret(626_273_431));
            processor.push(&frame(HANDSHAKE_BODY)).expect("handshake should decode");
            for _ in 0..2 {
                let body = msg(crate::artifact::COMPRESSED_API_ID, &compressed_payload(&compound));
                let result = processor.push(&encrypted_frame(&mut cipher, &body));
                if accepted {
                    let events = result.expect("exact entry limit should decode each frame");
                    assert_eq!(
                        events
                            .iter()
                            .map(|event| match event {
                                StreamEvent::Mails { entries, .. } => entries.len(),
                                _ => 0,
                            })
                            .sum::<usize>(),
                        crate::protobuf::MAX_MAIL_ENTRIES
                    );
                } else {
                    assert_eq!(result, Err(StreamError::Protocol));
                    break;
                }
            }
        }
    }

    #[test]
    fn processor_emits_login_context() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let login = [varint_field(1, 123_456), varint_field(2, 1804)].concat();
        let mut stream = frame(HANDSHAKE_BODY);
        stream.extend(encrypted_frame(&mut cipher, &msg(LOGIN_API_ID, &login)));

        let events = processor.push(&stream).expect("login frame should parse");

        assert_eq!(events, [StreamEvent::Login { player_id: 123_456, server_id: 1804 }]);
    }

    #[test]
    fn processor_emits_empty_terminal_mail_page() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let mut stream = frame(HANDSHAKE_BODY);
        stream.extend(encrypted_frame(&mut cipher, &msg(7921, &varint_field(2, 0))));

        let events = processor.push(&stream).expect("terminal mail page should parse");

        assert_eq!(
            events,
            [StreamEvent::Mails { server_id: None, entries: Vec::new(), remaining: Some(0) }]
        );
    }

    #[test]
    fn processor_preserves_cipher_state_across_split_and_ignored_frames() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let mut stream = frame(HANDSHAKE_BODY);
        stream.extend(encrypted_frame(&mut cipher, &msg(54, &[])));
        stream.extend(encrypted_frame(&mut cipher, &msg(7909, &carrier_payload(1))));

        for chunk in stream.chunks(3) {
            processor.push(chunk).expect("arbitrarily split stream should parse");
        }

        assert_eq!(processor.frame_index, 3);
    }

    #[test]
    fn processor_disables_on_malformed_decrypted_protobuf() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        processor.push(&frame(HANDSHAKE_BODY)).expect("handshake should parse");
        let mut cipher = StreamCipher::new(server_secret(626_273_431));

        let error = processor
            .push(&encrypted_frame(&mut cipher, &[0x80]))
            .expect_err("truncated protobuf should fail");

        assert_eq!(error, StreamError::Protocol);
    }

    #[test]
    fn processor_disables_on_declared_decompression_limit_violation() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);
        processor.push(&frame(HANDSHAKE_BODY)).expect("handshake should parse");
        let mut cipher = StreamCipher::new(server_secret(626_273_431));
        let mut wrapper = varint_field(
            1,
            u64::try_from(crate::protobuf::MAX_INFLATED_BYTES)
                .expect("limit should fit u64")
                .saturating_add(1),
        );
        wrapper.extend(bytes_field(2, &[0x78, 0x9c]));
        let body = msg(crate::artifact::ZMSG_API_ID, &wrapper);

        let error = processor
            .push(&encrypted_frame(&mut cipher, &body))
            .expect_err("oversized inflation declaration should fail");

        assert_eq!(error, StreamError::Decompression);
    }

    #[test]
    fn processor_rejects_non_handshake_first_frame() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut processor = ServerStreamProcessor::new(&artifact);

        let error = processor
            .push(&frame(&msg(1, &[])))
            .expect_err("first frame should not be a handshake");

        assert_eq!(error, StreamError::UnsupportedHandshake);
    }

    fn frame(body: &[u8]) -> Vec<u8> {
        let mut framed =
            Vec::from(u16::try_from(body.len()).expect("fixture should fit").to_be_bytes());
        framed.extend_from_slice(body);
        framed
    }

    fn encrypted_frame(cipher: &mut StreamCipher, body: &[u8]) -> Vec<u8> {
        let mut encrypted = body.to_vec();
        cipher.apply(&mut encrypted);
        frame(&encrypted)
    }

    fn carrier_payload(count: usize) -> Vec<u8> {
        let candidate = [bytes_field(1, b"mail-id"), bytes_field(9, b"Battle")].concat();
        (0..count).flat_map(|_| bytes_field(1, &candidate)).collect()
    }

    fn msg(api_id: u32, payload: &[u8]) -> Vec<u8> {
        [varint_field(1, u64::from(api_id)), bytes_field(2, payload)].concat()
    }

    fn compressed_payload(inflated: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(inflated).expect("fixture should compress");
        let compressed = encoder.finish().expect("fixture should finish");
        [
            varint_field(1, u64::try_from(inflated.len()).expect("fixture should fit u64")),
            bytes_field(2, &compressed),
        ]
        .concat()
    }

    fn varint_field(number: u64, value: u64) -> Vec<u8> {
        [encode_varint(number << 3), encode_varint(value)].concat()
    }

    fn bytes_field(number: u64, value: &[u8]) -> Vec<u8> {
        [
            encode_varint((number << 3) | 2),
            encode_varint(u64::try_from(value.len()).expect("fixture should fit u64")),
            value.to_vec(),
        ]
        .concat()
    }

    fn encode_varint(mut value: u64) -> Vec<u8> {
        let mut encoded = Vec::new();
        while value >= 0x80 {
            encoded.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        encoded.push(value as u8);
        encoded
    }

    #[test]
    fn decoder_error_wipes_and_permanently_retires_public_processor() {
        let artifact = RuntimeArtifact::test_fixture();
        let mut decoder = ServerStreamProcessor::new(&artifact);
        assert!(matches!(decoder.push(&[0, 1, 0xff]), Err(StreamError::UnsupportedHandshake)));
        assert!(decoder.failed);
        assert!(decoder.cipher.is_none());
        assert_eq!(decoder.buffered_bytes(), 0);
        assert!(decoder.body.is_empty());
        assert_eq!(decoder.push(&test_server_frames(0).remove(0)), Err(StreamError::Retired));
        assert!(!format!("{decoder:?}").contains("cipher"));
    }
}
