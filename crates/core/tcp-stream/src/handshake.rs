//! Parser for the unencrypted server handshake.

use serde::{Deserialize, Serialize};

/// API id used by the first usable unencrypted server frame.
pub const HANDSHAKE_API_ID: u64 = 8562;

/// Keys from the unencrypted server notification that starts a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    /// API id from field `1`; must match [`HANDSHAKE_API_ID`].
    pub api_id: u64,
    /// First stream key from nested field `2.1`.
    pub key1: u64,
    /// Second stream key from nested field `2.2`.
    pub key2: u64,
}

/// Parse the API `8562` notification body and pull out both stream keys.
///
/// The body is protobuf-like, but we only need three varints here, so a small
/// local cursor is enough.
pub fn parse_handshake(body: &[u8]) -> Option<Handshake> {
    let mut cursor = ProtoCursor::new(body);
    let mut api_id = None;
    let mut key1 = None;
    let mut key2 = None;

    while let Some((field, wire_type)) = cursor.next_tag()? {
        match (field, wire_type) {
            (1, WireType::Varint) => api_id = Some(cursor.read_varint()?),
            (2, WireType::LengthDelimited) => {
                let nested = cursor.read_len_delimited()?;
                let mut nested_cursor = ProtoCursor::new(nested);
                while let Some((nested_field, nested_wire_type)) = nested_cursor.next_tag()? {
                    match (nested_field, nested_wire_type) {
                        (1, WireType::Varint) => key1 = Some(nested_cursor.read_varint()?),
                        (2, WireType::Varint) => key2 = Some(nested_cursor.read_varint()?),
                        _ => nested_cursor.skip(nested_wire_type)?,
                    }
                }
            }
            _ => cursor.skip(wire_type)?,
        }
    }

    let api_id = api_id?;
    if api_id != HANDSHAKE_API_ID {
        return None;
    }

    Some(Handshake { api_id, key1: key1?, key2: key2? })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireType {
    Varint,
    Fixed64,
    LengthDelimited,
    Fixed32,
}

impl WireType {
    fn from_tag(tag: u64) -> Option<Self> {
        match tag & 0x07 {
            0 => Some(Self::Varint),
            1 => Some(Self::Fixed64),
            2 => Some(Self::LengthDelimited),
            5 => Some(Self::Fixed32),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ProtoCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ProtoCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn next_tag(&mut self) -> Option<Option<(u64, WireType)>> {
        if self.offset == self.bytes.len() {
            return Some(None);
        }

        let tag = self.read_varint()?;
        let field = tag >> 3;
        if field == 0 {
            return None;
        }

        Some(Some((field, WireType::from_tag(tag)?)))
    }

    fn read_varint(&mut self) -> Option<u64> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self.bytes.get(self.offset)?;
            self.offset += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn read_len_delimited(&mut self) -> Option<&'a [u8]> {
        let length = usize::try_from(self.read_varint()?).ok()?;
        let end = self.offset.checked_add(length)?;
        let bytes = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(bytes)
    }

    fn skip(&mut self, wire_type: WireType) -> Option<()> {
        match wire_type {
            WireType::Varint => {
                self.read_varint()?;
            }
            WireType::Fixed64 => {
                self.offset = self.offset.checked_add(8)?;
                self.bytes.get(..self.offset)?;
            }
            WireType::LengthDelimited => {
                self.read_len_delimited()?;
            }
            WireType::Fixed32 => {
                self.offset = self.offset.checked_add(4)?;
                self.bytes.get(..self.offset)?;
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HANDSHAKE_BODY: &[u8] = &[
        0x08, 0xf2, 0x42, 0x12, 0x0c, 0x08, 0x97, 0xd9, 0xd0, 0xaa, 0x02, 0x10, 0xd8, 0xb3, 0x98,
        0xf1, 0x03,
    ];

    #[test]
    fn parse_handshake_should_extract_api_and_keys() {
        let handshake = parse_handshake(HANDSHAKE_BODY);

        assert_eq!(
            handshake,
            Some(Handshake { api_id: 8562, key1: 626_273_431, key2: 1_042_684_376 })
        );
    }

    #[test]
    fn parse_handshake_should_reject_other_api_ids() {
        let handshake = parse_handshake(&[0x08, 0x01]);

        assert_eq!(handshake, None);
    }
}
