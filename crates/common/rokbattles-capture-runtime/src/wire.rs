//! Small versioned frames for one ordered, ephemeral capture transport.
//!
//! IDs are connection-local and never authenticate a user or prove mail ownership.
//! Reconnecting starts a new transport and new decoders; an offset is not a resume
//! capability. The transport must bound request rate/lifetime and apply backpressure.

use crate::{
    SERVER_PORTS,
    lifecycle::{AbortReason, Event},
};

const MAGIC: &[u8; 4] = b"RBC1";
const HEADER: usize = 27;
pub const MAX_CHUNK: usize = 16 * 1024;
/// One bounded reorder window plus one maximum captured TCP payload.
pub const MAX_EVENT_BYTES: usize = crate::reassembly::MAX_PENDING_BYTES + 65_535;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid capture frame")]
    Invalid,
    #[error("capture frame exceeds size limit")]
    TooLarge,
    #[error("capture transport ended inside a frame")]
    Truncated,
}

pub fn encode(event: &Event) -> Result<Vec<Vec<u8>>, Error> {
    let (kind, id, offset, value, bytes): (u8, u64, u64, u16, &[u8]) = match event {
        Event::Open { id, server_port } if SERVER_PORTS.contains(server_port) => {
            (1, *id, 0, *server_port, &[])
        }
        Event::Data { id, offset, bytes } if !bytes.is_empty() => (2, *id, *offset, 0, bytes),
        Event::Close { id } => (3, *id, 0, 0, &[]),
        Event::Abort { id, reason } => (4, *id, 0, abort_code(*reason), &[]),
        Event::Gap => (5, 0, 0, 0, &[]),
        Event::Keepalive => (6, 0, 0, 0, &[]),
        _ => return Err(Error::Invalid),
    };

    if bytes.len() > MAX_EVENT_BYTES {
        return Err(Error::TooLarge);
    }

    if kind < 5 && id == 0 {
        return Err(Error::Invalid);
    }

    let mut output = Vec::new();
    for index in 0..bytes.len().max(1).div_ceil(MAX_CHUNK) {
        let start = index.checked_mul(MAX_CHUNK).ok_or(Error::TooLarge)?;
        let end = start.saturating_add(MAX_CHUNK).min(bytes.len());
        let chunk = bytes.get(start..end).ok_or(Error::Invalid)?;
        let offset = offset
            .checked_add(u64::try_from(start).map_err(|_error| Error::TooLarge)?)
            .ok_or(Error::Invalid)?;
        let mut frame = Vec::with_capacity(HEADER + chunk.len());

        frame.extend(MAGIC);
        frame.push(kind);
        frame.extend(id.to_be_bytes());
        frame.extend(offset.to_be_bytes());
        frame.extend(value.to_be_bytes());
        frame.extend(u32::try_from(chunk.len()).map_err(|_error| Error::TooLarge)?.to_be_bytes());
        frame.extend(chunk);
        output.push(frame);
    }

    Ok(output)
}

/// At most one header+chunk is buffered, regardless of the incoming HTTP fragment.
#[derive(Default)]
pub struct Decoder {
    pending: Vec<u8>,
}

impl Decoder {
    pub fn push(
        &mut self,
        mut bytes: &[u8],
        mut receive: impl FnMut(Event) -> Result<(), Error>,
    ) -> Result<(), Error> {
        while !bytes.is_empty() {
            let target =
                if self.pending.len() < HEADER { HEADER } else { HEADER + length(&self.pending)? };
            let take = (target - self.pending.len()).min(bytes.len());
            self.pending.extend_from_slice(bytes.get(..take).ok_or(Error::Invalid)?);
            bytes = bytes.get(take..).ok_or(Error::Invalid)?;

            if self.pending.len() < HEADER {
                continue;
            }
            let payload_length = length(&self.pending)?;
            if self.pending.len() < HEADER + payload_length {
                continue;
            }

            let event = decode(&self.pending)?;
            self.pending.clear();
            receive(event)?;
        }

        Ok(())
    }

    pub fn finish(self) -> Result<(), Error> {
        if self.pending.is_empty() { Ok(()) } else { Err(Error::Truncated) }
    }
}

fn length(bytes: &[u8]) -> Result<usize, Error> {
    if bytes.get(..4) != Some(MAGIC) {
        return Err(Error::Invalid);
    }

    let size = u32::from_be_bytes(
        bytes.get(23..27).ok_or(Error::Truncated)?.try_into().map_err(|_error| Error::Invalid)?,
    ) as usize;
    if size > MAX_CHUNK { Err(Error::TooLarge) } else { Ok(size) }
}

fn decode(bytes: &[u8]) -> Result<Event, Error> {
    let id = u64::from_be_bytes(
        bytes.get(5..13).ok_or(Error::Truncated)?.try_into().map_err(|_error| Error::Invalid)?,
    );
    let offset = u64::from_be_bytes(
        bytes.get(13..21).ok_or(Error::Truncated)?.try_into().map_err(|_error| Error::Invalid)?,
    );
    let value = u16::from_be_bytes(
        bytes.get(21..23).ok_or(Error::Truncated)?.try_into().map_err(|_error| Error::Invalid)?,
    );
    let payload = bytes.get(HEADER..).ok_or(Error::Truncated)?;

    match (bytes.get(4), id, offset, value, payload.len()) {
        (Some(1), 1.., 0, port, 0) if SERVER_PORTS.contains(&port) => {
            Ok(Event::Open { id, server_port: port })
        }
        (Some(2), 1.., _, 0, 1..=MAX_CHUNK) => {
            Ok(Event::Data { id, offset, bytes: payload.to_vec() })
        }
        (Some(3), 1.., 0, 0, 0) => Ok(Event::Close { id }),
        (Some(4), 1.., 0, code, 0) => Ok(Event::Abort { id, reason: abort_reason(code)? }),
        (Some(5), 0, 0, 0, 0) => Ok(Event::Gap),
        (Some(6), 0, 0, 0, 0) => Ok(Event::Keepalive),
        _ => Err(Error::Invalid),
    }
}

fn abort_code(reason: AbortReason) -> u16 {
    match reason {
        AbortReason::Reset => 1,
        AbortReason::CaptureGap => 2,
        AbortReason::Reassembly => 3,
        AbortReason::AmbiguousGeneration => 4,
        AbortReason::MemoryLimit => 5,
        AbortReason::Idle => 6,
        AbortReason::InvalidHandshake => 7,
    }
}

fn abort_reason(code: u16) -> Result<AbortReason, Error> {
    match code {
        1 => Ok(AbortReason::Reset),
        2 => Ok(AbortReason::CaptureGap),
        3 => Ok(AbortReason::Reassembly),
        4 => Ok(AbortReason::AmbiguousGeneration),
        5 => Ok(AbortReason::MemoryLimit),
        6 => Ok(AbortReason::Idle),
        7 => Ok(AbortReason::InvalidHandshake),
        _ => Err(Error::Invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_network_chunking_preserves_events() {
        let expected = vec![
            Event::Open { id: 1, server_port: 5222 },
            Event::Data { id: 1, offset: 0, bytes: b"server bytes".to_vec() },
            Event::Close { id: 1 },
            Event::Abort { id: 2, reason: AbortReason::Reset },
            Event::Gap,
            Event::Keepalive,
        ];
        let bytes: Vec<u8> =
            expected.iter().flat_map(|event| encode(event).expect("encode")).flatten().collect();

        for size in 1..=HEADER + 1 {
            let mut decoder = Decoder::default();
            let mut actual = Vec::new();

            for fragment in bytes.chunks(size) {
                decoder
                    .push(fragment, |event| {
                        actual.push(event);
                        Ok(())
                    })
                    .expect("decode");
            }
            decoder.finish().expect("complete");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn large_contiguous_output_splits_without_resetting_offsets() {
        let frames = encode(&Event::Data { id: 1, offset: 5, bytes: vec![42; MAX_CHUNK * 2 + 1] })
            .expect("encode");
        assert_eq!(frames.len(), 3);
        let mut decoder = Decoder::default();
        let mut offsets = Vec::new();

        for frame in frames {
            decoder
                .push(&frame, |event| {
                    if let Event::Data { offset, bytes, .. } = event {
                        assert!(bytes.len() <= MAX_CHUNK);
                        offsets.push(offset);
                    }
                    Ok(())
                })
                .expect("decode");
        }
        assert_eq!(offsets, [5, 5 + MAX_CHUNK as u64, 5 + 2 * MAX_CHUNK as u64]);
    }

    #[test]
    fn rejects_oversized_events_before_copying_frames() {
        encode(&Event::Data { id: 1, offset: 0, bytes: vec![0; MAX_EVENT_BYTES] })
            .expect("exact maximum should encode");
        assert_eq!(
            encode(&Event::Data { id: 1, offset: 0, bytes: vec![0; MAX_EVENT_BYTES + 1] }),
            Err(Error::TooLarge)
        );
        assert_eq!(
            encode(&Event::Data { id: 1, offset: u64::MAX, bytes: vec![0; MAX_CHUNK + 1] }),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn rejects_huge_lengths_at_header_and_truncated_eof() {
        let mut frame = encode(&Event::Keepalive).expect("encode").remove(0);
        frame.get_mut(23..27).expect("length").copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(Decoder::default().push(&frame, |_| Ok(())), Err(Error::TooLarge));

        let mut decoder = Decoder::default();
        decoder.push(b"RBC", |_| Ok(())).expect("partial");
        assert_eq!(decoder.finish(), Err(Error::Truncated));
    }

    #[test]
    fn rejects_other_ports_zero_ids_and_invalid_abort_codes() {
        assert_eq!(encode(&Event::Open { id: 1, server_port: 443 }), Err(Error::Invalid));
        assert_eq!(encode(&Event::Close { id: 0 }), Err(Error::Invalid));
        let mut frame =
            encode(&Event::Abort { id: 1, reason: AbortReason::Reset }).expect("encode").remove(0);
        frame.get_mut(21..23).expect("reason").copy_from_slice(&999_u16.to_be_bytes());
        assert_eq!(Decoder::default().push(&frame, |_| Ok(())), Err(Error::Invalid));
    }
}
