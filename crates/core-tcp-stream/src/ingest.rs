//! JSON shape used for temporary TCP stream uploads.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::{handshake::Handshake, types::StreamId};

const MAX_FRAMES_PER_BATCH: usize = 4096;
const MAX_FRAME_BODY_BYTES: usize = 64 * 1024;

/// One server-to-client frame in an upload batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcpStreamFrameUpload {
    /// Zero-based frame number within the stream.
    pub index: u64,
    /// Base64 body bytes, with the two-byte stream length prefix removed.
    pub body_base64: String,
}

impl TcpStreamFrameUpload {
    /// Encode raw body bytes for upload.
    pub fn from_body(index: u64, body: &[u8]) -> Self {
        Self { index, body_base64: STANDARD.encode(body) }
    }

    /// Decode the frame body back to bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FrameBodyError`] if the base64 is invalid or the decoded body
    /// is larger than ingress accepts.
    pub fn body(&self) -> Result<Vec<u8>, FrameBodyError> {
        let bytes =
            STANDARD.decode(&self.body_base64).map_err(|_| FrameBodyError::InvalidBase64)?;
        if bytes.len() > MAX_FRAME_BODY_BYTES {
            return Err(FrameBodyError::TooLarge {
                length: bytes.len(),
                max: MAX_FRAME_BODY_BYTES,
            });
        }
        Ok(bytes)
    }
}

/// A batch of frames from one TCP stream.
///
/// The desktop app repeats the stream id and handshake in every batch. That
/// keeps each upload usable on its own, even when a long session is split across
/// timed flushes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcpStreamBatch {
    /// App-generated id for this capture session.
    pub capture_id: String,
    /// Zero-based batch number for this capture session.
    pub batch_index: u64,
    /// TCP connection tuple.
    pub stream: StreamId,
    /// Parsed keys from the API `8562` handshake.
    pub handshake: Handshake,
    /// Server-to-client frames in this batch.
    pub frames: Vec<TcpStreamFrameUpload>,
}

impl TcpStreamBatch {
    /// Check the batch and decode its frame bodies.
    ///
    /// # Errors
    ///
    /// Returns [`TcpStreamBatchValidation`] if required metadata is missing, the
    /// batch is empty, frame indexes are out of order, or a body cannot decode.
    pub fn validate(&self) -> Result<ValidatedTcpStreamBatch, TcpStreamBatchValidation> {
        if self.capture_id.trim().is_empty() {
            return Err(TcpStreamBatchValidation::MissingCaptureId);
        }
        if self.frames.is_empty() {
            return Err(TcpStreamBatchValidation::EmptyFrames);
        }
        if self.frames.len() > MAX_FRAMES_PER_BATCH {
            return Err(TcpStreamBatchValidation::TooManyFrames {
                count: self.frames.len(),
                max: MAX_FRAMES_PER_BATCH,
            });
        }

        let mut frames = Vec::with_capacity(self.frames.len());
        let mut previous_index = None;
        for frame in &self.frames {
            if let Some(previous) = previous_index
                && frame.index <= previous
            {
                return Err(TcpStreamBatchValidation::NonMonotonicFrameIndex {
                    previous,
                    current: frame.index,
                });
            }
            previous_index = Some(frame.index);
            frames.push(ValidatedTcpStreamFrame { index: frame.index, body: frame.body()? });
        }

        Ok(ValidatedTcpStreamBatch {
            capture_id: self.capture_id.clone(),
            batch_index: self.batch_index,
            stream: self.stream.clone(),
            handshake: self.handshake,
            frames,
        })
    }
}

/// TCP stream batch after validation and base64 decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTcpStreamBatch {
    /// App-generated id for this capture session.
    pub capture_id: String,
    /// Zero-based batch number for this capture session.
    pub batch_index: u64,
    /// TCP connection tuple.
    pub stream: StreamId,
    /// Parsed keys from the API `8562` handshake.
    pub handshake: Handshake,
    /// Decoded server-to-client frames.
    pub frames: Vec<ValidatedTcpStreamFrame>,
}

/// Decoded server-to-client frame from an accepted batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTcpStreamFrame {
    /// Zero-based frame number within the stream.
    pub index: u64,
    /// Raw frame body with the two-byte stream length prefix removed.
    pub body: Vec<u8>,
}

/// Problems found while decoding a frame body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameBodyError {
    /// Body was not valid base64.
    #[error("frame body is not valid base64")]
    InvalidBase64,
    /// Decoded body was too large.
    #[error("frame body length {length} exceeds maximum {max}")]
    TooLarge {
        /// Decoded body length.
        length: usize,
        /// Largest body accepted by ingress.
        max: usize,
    },
}

/// Problems found while checking a batch.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TcpStreamBatchValidation {
    /// Capture id was empty.
    #[error("missing capture id")]
    MissingCaptureId,
    /// Batch did not include any frames.
    #[error("tcp stream batch must contain at least one frame")]
    EmptyFrames,
    /// Batch had more frames than ingress accepts.
    #[error("tcp stream batch has {count} frames; maximum is {max}")]
    TooManyFrames {
        /// Submitted frame count.
        count: usize,
        /// Largest allowed frame count.
        max: usize,
    },
    /// Frame indexes were not strictly increasing.
    #[error("tcp stream frame index {current} is not greater than previous index {previous}")]
    NonMonotonicFrameIndex {
        /// Previous frame index.
        previous: u64,
        /// Current frame index.
        current: u64,
    },
    /// One frame body could not be decoded.
    #[error(transparent)]
    FrameBody(#[from] FrameBodyError),
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    use crate::types::CLIENT_PORT;

    #[test]
    fn tcp_stream_frame_upload_should_roundtrip_body() {
        let frame = TcpStreamFrameUpload::from_body(3, &[0xaa, 0xbb]);

        let body = frame.body().expect("body should decode");

        assert_eq!(body, vec![0xaa, 0xbb]);
    }

    #[test]
    fn tcp_stream_batch_should_validate_supported_request() {
        let batch = sample_batch(vec![TcpStreamFrameUpload::from_body(0, &[0x01])]);

        let validated = batch.validate().expect("batch should validate");

        assert_eq!(validated.frames.len(), 1);
    }

    #[test]
    fn tcp_stream_batch_should_reject_empty_frames() {
        let batch = sample_batch(Vec::new());

        let error = batch.validate().expect_err("empty frames should fail");

        assert_eq!(error, TcpStreamBatchValidation::EmptyFrames);
    }

    #[test]
    fn tcp_stream_batch_should_reject_non_monotonic_frame_indexes() {
        let batch = sample_batch(vec![
            TcpStreamFrameUpload::from_body(2, &[0x01]),
            TcpStreamFrameUpload::from_body(2, &[0x02]),
        ]);

        let error = batch.validate().expect_err("duplicate indexes should fail");

        assert_eq!(
            error,
            TcpStreamBatchValidation::NonMonotonicFrameIndex { previous: 2, current: 2 }
        );
    }

    fn sample_batch(frames: Vec<TcpStreamFrameUpload>) -> TcpStreamBatch {
        TcpStreamBatch {
            capture_id: "capture-1".to_string(),
            batch_index: 0,
            stream: StreamId {
                client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
                client_port: 56_380,
                server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
                server_port: CLIENT_PORT,
            },
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            frames,
        }
    }
}
