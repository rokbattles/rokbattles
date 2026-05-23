//! Reader for two-byte length-prefixed stream frames.

/// Largest body length that fits in the two-byte prefix.
pub const MAX_FRAME_BODY_LEN: usize = u16::MAX as usize;

/// Errors from the frame reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameReadError {
    /// The frame prefix declared a body that is too large.
    #[error("frame body length {length} exceeds maximum {max}")]
    BodyTooLarge {
        /// Length from the wire prefix.
        length: usize,
        /// Largest body length accepted by the reader.
        max: usize,
    },
}

/// Incremental reader for the stream frame format.
#[derive(Debug, Default)]
pub struct FrameReader {
    buffer: Vec<u8>,
}

impl FrameReader {
    /// Create a reader with no buffered bytes.
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Add TCP payload bytes and return any complete frame bodies.
    ///
    /// A TCP segment can contain several frames, or only part of one. Complete
    /// bodies are returned immediately; trailing partial bytes stay in the
    /// buffer for the next call.
    ///
    /// # Errors
    ///
    /// Returns [`FrameReadError`] if the length prefix cannot be accepted.
    pub fn push(&mut self, payload: &[u8]) -> Result<Vec<Vec<u8>>, FrameReadError> {
        self.buffer.extend_from_slice(payload);

        let mut frames = Vec::new();
        loop {
            if self.buffer.len() < 2 {
                return Ok(frames);
            }

            let length = usize::from(u16::from_be_bytes([self.buffer[0], self.buffer[1]]));
            if length > MAX_FRAME_BODY_LEN {
                self.buffer.clear();
                return Err(FrameReadError::BodyTooLarge { length, max: MAX_FRAME_BODY_LEN });
            }

            let Some(end) = 2usize.checked_add(length) else {
                self.buffer.clear();
                return Err(FrameReadError::BodyTooLarge { length, max: MAX_FRAME_BODY_LEN });
            };
            if self.buffer.len() < end {
                return Ok(frames);
            }

            // Drop the two-byte transport prefix before handing the body to
            // callers. Draining also leaves the buffer ready for the next frame.
            let body = self.buffer[2..end].to_vec();
            self.buffer.drain(..end);
            frames.push(body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_should_wait_for_partial_frame() {
        let mut reader = FrameReader::new();

        let frames = reader.push(&[0x00, 0x03, 0xaa]).expect("frame read should succeed");

        assert!(frames.is_empty());
    }

    #[test]
    fn push_should_return_completed_partial_frame_after_next_payload() {
        let mut reader = FrameReader::new();
        let _ = reader.push(&[0x00, 0x03, 0xaa]).expect("first push should succeed");

        let frames = reader.push(&[0xbb, 0xcc]).expect("second push should succeed");

        assert_eq!(frames, vec![vec![0xaa, 0xbb, 0xcc]]);
    }

    #[test]
    fn push_should_split_coalesced_frames() {
        let mut reader = FrameReader::new();

        let frames = reader
            .push(&[0x00, 0x01, 0xaa, 0x00, 0x02, 0xbb, 0xcc])
            .expect("coalesced frames should parse");

        assert_eq!(frames, vec![vec![0xaa], vec![0xbb, 0xcc]]);
    }
}
