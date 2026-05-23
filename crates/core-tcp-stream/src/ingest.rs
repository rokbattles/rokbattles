//! JSON shape used for temporary TCP stream uploads.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::{
    handshake::Handshake,
    types::{Direction, StreamId},
};

const MAX_FRAGMENTS_PER_BATCH: usize = 8192;
const MAX_FRAGMENT_BYTES: usize = 256 * 1024;

/// One TCP payload fragment captured after a stream is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcpStreamFragmentUpload {
    /// Zero-based fragment number within the accepted stream.
    pub index: u64,
    /// Direction of the TCP payload bytes.
    pub direction: Direction,
    /// Base64 TCP payload bytes, including any frame length prefixes.
    pub payload_base64: String,
}

impl TcpStreamFragmentUpload {
    /// Encode raw TCP payload bytes for upload.
    pub fn from_payload(index: u64, direction: Direction, payload: &[u8]) -> Self {
        Self { index, direction, payload_base64: STANDARD.encode(payload) }
    }

    /// Decode the fragment payload back to bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FragmentPayloadError`] if the base64 is invalid or the decoded
    /// payload is larger than ingress accepts.
    pub fn payload(&self) -> Result<Vec<u8>, FragmentPayloadError> {
        let bytes = STANDARD
            .decode(&self.payload_base64)
            .map_err(|_| FragmentPayloadError::InvalidBase64)?;
        if bytes.len() > MAX_FRAGMENT_BYTES {
            return Err(FragmentPayloadError::TooLarge {
                length: bytes.len(),
                max: MAX_FRAGMENT_BYTES,
            });
        }
        Ok(bytes)
    }
}

/// A batch of TCP payload fragments from one stream.
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
    /// True when this is the final upload for the stream.
    pub stream_ended: bool,
    /// TCP connection tuple.
    pub stream: StreamId,
    /// Parsed keys from the API `8562` handshake.
    pub handshake: Handshake,
    /// Ordered TCP payload fragments for stateful stream processing.
    pub fragments: Vec<TcpStreamFragmentUpload>,
}

impl TcpStreamBatch {
    /// Check the batch and decode its fragment payloads.
    ///
    /// # Errors
    ///
    /// Returns [`TcpStreamBatchValidation`] if required metadata is missing, the
    /// batch is empty, fragment indexes are out of order, or a payload cannot
    /// decode.
    pub fn validate(&self) -> Result<ValidatedTcpStreamBatch, TcpStreamBatchValidation> {
        if self.capture_id.trim().is_empty() {
            return Err(TcpStreamBatchValidation::MissingCaptureId);
        }
        if self.fragments.is_empty() && !self.stream_ended {
            return Err(TcpStreamBatchValidation::EmptyFragments);
        }
        if self.fragments.len() > MAX_FRAGMENTS_PER_BATCH {
            return Err(TcpStreamBatchValidation::TooManyFragments {
                count: self.fragments.len(),
                max: MAX_FRAGMENTS_PER_BATCH,
            });
        }

        let mut fragments = Vec::with_capacity(self.fragments.len());
        let mut previous_fragment_index = None;
        for fragment in &self.fragments {
            if let Some(previous) = previous_fragment_index
                && fragment.index <= previous
            {
                return Err(TcpStreamBatchValidation::NonMonotonicFragmentIndex {
                    previous,
                    current: fragment.index,
                });
            }
            previous_fragment_index = Some(fragment.index);
            fragments.push(ValidatedTcpStreamFragment {
                index: fragment.index,
                direction: fragment.direction,
                payload: fragment.payload()?,
            });
        }

        Ok(ValidatedTcpStreamBatch {
            capture_id: self.capture_id.clone(),
            batch_index: self.batch_index,
            stream_ended: self.stream_ended,
            stream: self.stream.clone(),
            handshake: self.handshake,
            fragments,
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
    /// True when this is the final upload for the stream.
    pub stream_ended: bool,
    /// TCP connection tuple.
    pub stream: StreamId,
    /// Parsed keys from the API `8562` handshake.
    pub handshake: Handshake,
    /// Decoded TCP payload fragments.
    pub fragments: Vec<ValidatedTcpStreamFragment>,
}

/// Decoded TCP payload fragment from an accepted stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTcpStreamFragment {
    /// Zero-based fragment number within the accepted stream.
    pub index: u64,
    /// Direction of the TCP payload bytes.
    pub direction: Direction,
    /// TCP payload bytes, including any frame length prefixes.
    pub payload: Vec<u8>,
}

/// Problems found while decoding a TCP fragment payload.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FragmentPayloadError {
    /// Payload was not valid base64.
    #[error("tcp stream fragment payload is not valid base64")]
    InvalidBase64,
    /// Decoded payload was too large.
    #[error("tcp stream fragment payload length {length} exceeds maximum {max}")]
    TooLarge {
        /// Decoded payload length.
        length: usize,
        /// Largest payload accepted by ingress.
        max: usize,
    },
}

/// Problems found while checking a batch.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TcpStreamBatchValidation {
    /// Capture id was empty.
    #[error("missing capture id")]
    MissingCaptureId,
    /// Batch did not include any fragments.
    #[error("tcp stream batch must contain at least one fragment")]
    EmptyFragments,
    /// Batch had more fragments than ingress accepts.
    #[error("tcp stream batch has {count} fragments; maximum is {max}")]
    TooManyFragments {
        /// Submitted fragment count.
        count: usize,
        /// Largest allowed fragment count.
        max: usize,
    },
    /// Fragment indexes were not strictly increasing.
    #[error("tcp stream fragment index {current} is not greater than previous index {previous}")]
    NonMonotonicFragmentIndex {
        /// Previous fragment index.
        previous: u64,
        /// Current fragment index.
        current: u64,
    },
    /// One fragment payload could not be decoded.
    #[error(transparent)]
    FragmentPayload(#[from] FragmentPayloadError),
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    use crate::types::CLIENT_PORT;

    #[test]
    fn tcp_stream_fragment_upload_should_roundtrip_payload() {
        let fragment = TcpStreamFragmentUpload::from_payload(
            3,
            Direction::ServerToClient,
            &[0x00, 0x01, 0xaa],
        );

        let payload = fragment.payload().expect("payload should decode");

        assert_eq!(payload, vec![0x00, 0x01, 0xaa]);
    }

    #[test]
    fn tcp_stream_batch_should_validate_supported_request() {
        let batch = sample_batch();

        let validated = batch.validate().expect("batch should validate");

        assert_eq!(validated.fragments.len(), 1);
    }

    #[test]
    fn tcp_stream_batch_should_reject_empty_batch() {
        let mut batch = sample_batch();
        batch.fragments.clear();
        batch.stream_ended = false;

        let error = batch.validate().expect_err("empty batch should fail");

        assert_eq!(error, TcpStreamBatchValidation::EmptyFragments);
    }

    #[test]
    fn tcp_stream_batch_should_allow_empty_final_batch() {
        let mut batch = sample_batch();
        batch.fragments.clear();
        batch.stream_ended = true;

        let validated = batch.validate().expect("final empty batch should validate");

        assert!(validated.stream_ended);
    }

    #[test]
    fn tcp_stream_batch_should_reject_non_monotonic_fragment_indexes() {
        let mut batch = sample_batch();
        batch.fragments = vec![
            TcpStreamFragmentUpload::from_payload(2, Direction::ServerToClient, &[0x01]),
            TcpStreamFragmentUpload::from_payload(2, Direction::ClientToServer, &[0x02]),
        ];

        let error = batch.validate().expect_err("duplicate fragment indexes should fail");

        assert_eq!(
            error,
            TcpStreamBatchValidation::NonMonotonicFragmentIndex { previous: 2, current: 2 }
        );
    }

    fn sample_batch() -> TcpStreamBatch {
        TcpStreamBatch {
            capture_id: "capture-1".to_string(),
            batch_index: 0,
            stream_ended: false,
            stream: StreamId {
                client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
                client_port: 56_380,
                server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
                server_port: CLIENT_PORT,
            },
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            fragments: vec![TcpStreamFragmentUpload::from_payload(
                0,
                Direction::ServerToClient,
                &[0x00, 0x01, 0xaa],
            )],
        }
    }
}
