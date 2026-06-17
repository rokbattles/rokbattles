//! Stream decryption and length-prefixed frame reconstruction.

pub use tcp_stream::Direction;
use tcp_stream::framing::MAX_FRAME_BODY_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFragment {
    pub index: u64,
    pub direction: Direction,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedFrame {
    pub direction: Direction,
    pub index: u64,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub struct StreamDecryptor {
    client: DirectionState,
    server: DirectionState,
}

impl StreamDecryptor {
    pub fn new(key1: u64, key2: u64) -> Self {
        Self {
            client: DirectionState::new(
                Direction::ClientToServer,
                generate_secret(key2, SecretMode::Client),
                false,
            ),
            server: DirectionState::new(
                Direction::ServerToClient,
                generate_secret(key1, SecretMode::Server),
                true,
            ),
        }
    }

    pub fn push(&mut self, fragment: &RawFragment) -> Result<Vec<DecryptedFrame>, String> {
        match fragment.direction {
            Direction::ClientToServer => self.client.push(&fragment.payload),
            Direction::ServerToClient => self.server.push(&fragment.payload),
        }
    }
}

#[derive(Debug)]
struct DirectionState {
    direction: Direction,
    prefix: Vec<u8>,
    extended: Vec<u8>,
    remaining: Option<usize>,
    length: Option<usize>,
    body: Vec<u8>,
    frame_index: u64,
    cipher: StreamCipher,
    skip_plain_first: bool,
}

impl DirectionState {
    fn new(direction: Direction, seed: u32, skip_plain_first: bool) -> Self {
        Self {
            direction,
            prefix: Vec::new(),
            extended: Vec::new(),
            remaining: None,
            length: None,
            body: Vec::new(),
            frame_index: 0,
            cipher: StreamCipher::new(seed),
            skip_plain_first,
        }
    }

    fn push(&mut self, payload: &[u8]) -> Result<Vec<DecryptedFrame>, String> {
        let mut pos = 0usize;
        let mut frames = Vec::new();
        while pos < payload.len() {
            if self.remaining.is_none() {
                self.read_prefix(payload, &mut pos)?;
                if self.remaining.is_none() {
                    break;
                }
            }

            let remaining = self.remaining.ok_or_else(|| "missing frame length".to_string())?;
            let available = payload.len().saturating_sub(pos);
            let take = remaining.min(available);
            let Some(encrypted) = payload.get(pos..pos.saturating_add(take)) else {
                return Err("fragment slice was out of bounds".to_string());
            };

            let decrypted = if self.skip_plain_first
                && self.direction == Direction::ServerToClient
                && self.frame_index == 0
            {
                encrypted.to_vec()
            } else {
                self.cipher.apply(encrypted)?
            };
            self.body.extend_from_slice(&decrypted);
            pos = pos.saturating_add(take);
            let next_remaining = remaining.saturating_sub(take);
            self.remaining = Some(next_remaining);

            if next_remaining == 0 {
                let body = std::mem::take(&mut self.body);
                frames.push(DecryptedFrame {
                    direction: self.direction,
                    index: self.frame_index,
                    body,
                });
                self.frame_index = self.frame_index.saturating_add(1);
                self.length = None;
                self.remaining = None;
            }
        }
        Ok(frames)
    }

    fn read_prefix(&mut self, payload: &[u8], pos: &mut usize) -> Result<(), String> {
        if self.prefix.len() < 2 {
            take_into(&mut self.prefix, 2, payload, pos);
            if self.prefix.len() < 2 {
                return Ok(());
            }
        }

        let prefix: [u8; 2] = self
            .prefix
            .as_slice()
            .try_into()
            .map_err(|_error| "invalid frame prefix length".to_string())?;
        let short = u16::from_be_bytes(prefix);
        let length = if short == u16::MAX {
            take_into(&mut self.extended, 4, payload, pos);
            if self.extended.len() < 4 {
                return Ok(());
            }
            let extended: [u8; 4] = self
                .extended
                .as_slice()
                .try_into()
                .map_err(|_error| "invalid extended frame prefix length".to_string())?;
            u32::from_be_bytes(extended) as usize
        } else {
            usize::from(short)
        };

        self.prefix.clear();
        self.extended.clear();
        if length > MAX_FRAME_BODY_LEN {
            return Err(format!("frame body length {length} exceeds maximum {MAX_FRAME_BODY_LEN}"));
        }
        self.length = Some(length);
        self.remaining = Some(length);
        Ok(())
    }
}

fn take_into(target: &mut Vec<u8>, target_len: usize, payload: &[u8], pos: &mut usize) {
    let needed = target_len.saturating_sub(target.len());
    let available = payload.len().saturating_sub(*pos);
    let take = needed.min(available);
    if let Some(bytes) = payload.get(*pos..pos.saturating_add(take)) {
        target.extend_from_slice(bytes);
        *pos = pos.saturating_add(take);
    }
}

#[derive(Debug, Clone, Copy)]
enum SecretMode {
    Server,
    Client,
}

fn generate_secret(value: u64, mode: SecretMode) -> u32 {
    let secret = match mode {
        SecretMode::Server => (value >> 1).saturating_add(0x400),
        SecretMode::Client => (value / 3).saturating_add(0x2766),
    };
    (secret & u64::from(u32::MAX)) as u32
}

#[derive(Debug)]
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
        self.words.get(offset / 4).copied().unwrap_or(0)
    }

    fn set(&mut self, offset: usize, value: u32) {
        if let Some(slot) = self.words.get_mut(offset / 4) {
            *slot = value;
        }
    }

    fn array_get(&self, base: usize, index: u32) -> u32 {
        self.get(base + 4 * usize::try_from(index & 0x3f).unwrap_or(0))
    }

    fn array_set(&mut self, base: usize, index: u32, value: u32) {
        self.set(base + 4 * usize::try_from(index & 0x3f).unwrap_or(0), value);
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
            self.set(base - 8, 0x3f);
            self.set(base - 4, 0);
        }
        self.set(0x04, 0x1000);
    }

    fn twist(value: u32) -> u32 {
        let mut ecx = (value << 1) ^ value;
        ecx = (ecx << 1) ^ value;
        ecx = (ecx << 2) ^ value;
        ecx = (ecx << 2) ^ value;
        ecx = (ecx << 25) ^ value;
        (ecx & 0x8000_0000) | (value >> 1)
    }

    fn seed_feedback(value: u32) -> u32 {
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
        let index = (self.get(index_offset).wrapping_add(1)) & 0x3f;
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

    fn apply(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = data.to_vec();
        let mut pos = 0usize;
        while pos < out.len() {
            let mut offset = self.get(0x04) as usize;
            if offset >= 0x1000 {
                self.refill();
                offset = 0;
            }
            let n = (0x1000usize.saturating_sub(offset)).min(out.len().saturating_sub(pos));
            let stream = self.keystream(offset, n);
            let end = pos.saturating_add(n);
            let Some(bytes) = out.get_mut(pos..end) else {
                return Err("cipher output slice was out of bounds".to_string());
            };
            for (byte, key) in bytes.iter_mut().zip(stream) {
                *byte ^= key;
            }
            self.set(0x04, u32::try_from(offset.saturating_add(n)).unwrap_or(u32::MAX));
            pos = end;
        }
        Ok(out)
    }

    fn keystream(&self, offset: usize, len: usize) -> Vec<u8> {
        let word_start = offset / 4;
        let word_end = (offset.saturating_add(len).saturating_add(3)) / 4;
        let mut stream = Vec::with_capacity(word_end.saturating_sub(word_start) * 4);
        for word_index in word_start..word_end {
            stream.extend_from_slice(&self.get(0x344 + 4 * word_index).to_le_bytes());
        }
        let start = offset % 4;
        stream.into_iter().skip(start).take(len).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HANDSHAKE_FRAME: &[u8] = &[
        0x00, 0x11, 0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8,
        0xb3, 0x98, 0xf1, 0x03,
    ];

    #[test]
    fn stream_decryptor_emits_plain_first_server_frame() {
        let mut decryptor = StreamDecryptor::new(626_273_431, 1_042_684_376);
        let fragment = RawFragment {
            index: 0,
            direction: Direction::ServerToClient,
            payload: HANDSHAKE_FRAME.to_vec(),
        };

        let frames = decryptor.push(&fragment).expect("fragment should parse");

        assert_eq!(frames.len(), 1);
        assert_eq!(frames.first().map(|frame| frame.body.as_slice()), Some(&HANDSHAKE_FRAME[2..]));
    }

    #[test]
    fn stream_decryptor_decodes_first_encrypted_server_frame() {
        let mut decryptor = StreamDecryptor::new(626_273_431, 1_042_684_376);
        let handshake = RawFragment {
            index: 0,
            direction: Direction::ServerToClient,
            payload: HANDSHAKE_FRAME.to_vec(),
        };
        decryptor.push(&handshake).expect("handshake should parse");
        let encrypted = RawFragment {
            index: 1,
            direction: Direction::ServerToClient,
            payload: vec![0x00, 0x04, 0xef, 0xad, 0x88, 0xc8],
        };

        let frames = decryptor.push(&encrypted).expect("encrypted frame should parse");

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames.first().map(|frame| frame.body.as_slice()),
            Some(&[0x08, 0x36, 0x12, 0x00][..])
        );
    }

    #[test]
    fn stream_decryptor_rejects_too_large_extended_frame() {
        let mut decryptor = StreamDecryptor::new(626_273_431, 1_042_684_376);
        let length = u32::try_from(MAX_FRAME_BODY_LEN + 1).expect("max frame length should fit");
        let mut payload = Vec::from(u16::MAX.to_be_bytes());
        payload.extend_from_slice(&length.to_be_bytes());
        let fragment = RawFragment { index: 0, direction: Direction::ServerToClient, payload };

        let error = decryptor.push(&fragment).expect_err("oversized frame should fail");

        assert_eq!(
            error,
            format!("frame body length {} exceeds maximum {MAX_FRAME_BODY_LEN}", length)
        );
    }
}
