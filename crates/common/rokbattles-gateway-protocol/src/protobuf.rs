//! Bounded protobuf field parsing and transport-wrapper unwrapping.

use std::{borrow::Cow, io::Read};

use flate2::read::ZlibDecoder;

use crate::artifact::{COMPRESSED_API_ID, CompressionSchema, RuntimeArtifact, ZMSG_API_ID};

pub(crate) const MAX_INFLATED_BYTES: usize = 25 * 1024 * 1024;
// Tiny compound members must not amplify one bounded frame into millions of
// allocations or callbacks. This is a per-frame work bound, not a session quota.
pub(crate) const MAX_COMPOUND_MESSAGES: usize = 4096;
// Bound Vec metadata amplification from tiny repeated entities within one frame.
pub(crate) const MAX_MAIL_ENTRIES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ProtocolError {
    #[error("protobuf input ended before a field was complete")]
    Truncated,
    #[error("protobuf varint exceeded 64 bits")]
    VarintOverflow,
    #[error("protobuf field number was zero")]
    ZeroFieldNumber,
    #[error("protobuf wire type is unsupported")]
    UnsupportedWireType,
    #[error("protobuf field has an incompatible wire type")]
    WrongWireType,
    #[error("protobuf message is missing a required protocol field")]
    MissingField,
    #[error("protobuf integer does not fit the protocol field")]
    IntegerOutOfRange,
    #[error("compressed payload declares more than the inflation limit")]
    DeclaredInflationTooLarge,
    #[error("compressed payload exceeded the inflation limit")]
    InflationTooLarge,
    #[error("compressed payload length did not match its declaration")]
    InflationLengthMismatch,
    #[error("compressed payload could not be inflated")]
    Inflate,
    #[error("compressed wrapper contained an unexpected message")]
    UnexpectedWrappedMessage,
    #[error("compound message contains too many members")]
    TooManyMessages,
    #[error("frame contains too many mail entries")]
    TooManyMailEntries,
}

#[derive(Debug)]
pub(crate) struct EffectiveMessage<'a> {
    pub(crate) api_id: u32,
    pub(crate) payload: Cow<'a, [u8]>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MailCandidates {
    pub(crate) entries: Vec<Vec<u8>>,
    pub(crate) server_id: Option<i32>,
    pub(crate) remaining: Option<usize>,
}

pub(crate) fn parse_handshake(
    body: &[u8],
    artifact: &RuntimeArtifact,
) -> Result<(u64, u64), ProtocolError> {
    let msg = parse_msg(body, artifact)?;
    if msg.api_id != crate::artifact::HANDSHAKE_API_ID {
        return Err(ProtocolError::UnexpectedWrappedMessage);
    }
    let key1 = required_varint(&msg.payload, artifact.protocol.handshake_key1_field)?;
    let key2 = required_varint(&msg.payload, artifact.protocol.handshake_key2_field)?;
    Ok((key1, key2))
}

pub(crate) fn visit_messages(
    body: &[u8],
    artifact: &RuntimeArtifact,
    mut visit: impl FnMut(EffectiveMessage<'_>) -> Result<(), ProtocolError>,
) -> Result<(), ProtocolError> {
    let outer = parse_msg(body, artifact)?;
    match outer.api_id {
        COMPRESSED_API_ID => {
            let inflated = inflate_wrapper(&outer.payload, artifact.protocol.compressed)?;
            // CompressedMsg contains CompoundMsg, whose Messages field is
            // repeated. Visit every member in wire order; taking the last bytes
            // field silently loses earlier login and mail messages.
            let mut cursor = FieldCursor::new(&inflated);
            let mut count = 0;
            while let Some(field) = cursor.next()? {
                if field.number == artifact.protocol.compound_messages_field {
                    count += 1;
                    if count > MAX_COMPOUND_MESSAGES {
                        return Err(ProtocolError::TooManyMessages);
                    }
                    let FieldValue::LengthDelimited(body) = field.value else {
                        return Err(ProtocolError::WrongWireType);
                    };
                    visit(parse_msg(body, artifact)?)?;
                }
            }
            Ok(())
        }
        ZMSG_API_ID => {
            let inflated = inflate_wrapper(&outer.payload, artifact.protocol.zmsg)?;
            visit(parse_msg(&inflated, artifact)?)
        }
        _ => visit(outer),
    }
}

pub(crate) fn mail_candidates(
    payload: &[u8],
    artifact: &RuntimeArtifact,
    api_id: u32,
    entry_budget: usize,
) -> Result<MailCandidates, ProtocolError> {
    let carrier = artifact.carriers.get(&api_id).ok_or(ProtocolError::UnexpectedWrappedMessage)?;
    let mut entries = Vec::new();
    let mut server_id = None;
    let mut remaining = None;
    let mut cursor = FieldCursor::new(payload);
    while let Some(field) = cursor.next()? {
        if !carrier.shape.accepts(field.number, field.wire) {
            return Err(ProtocolError::WrongWireType);
        }
        if Some(field.number) == carrier.left_count_field {
            let FieldValue::Varint(value) = field.value else {
                return Err(ProtocolError::WrongWireType);
            };
            remaining = Some(
                usize::try_from(decode_int32(value)?)
                    .map_err(|_error| ProtocolError::IntegerOutOfRange)?,
            );
            continue;
        }
        if field.number != carrier.entity_field {
            continue;
        }
        let FieldValue::LengthDelimited(candidate) = field.value else {
            return Err(ProtocolError::WrongWireType);
        };
        if entries.len() >= entry_budget.min(MAX_MAIL_ENTRIES) {
            return Err(ProtocolError::TooManyMailEntries);
        }
        let candidate_server_id = validate_mail_entity(candidate, artifact)?;
        if server_id.is_none() {
            server_id = candidate_server_id.filter(|server_id| *server_id != 0);
        }
        entries.push(candidate.to_vec());
    }
    Ok(MailCandidates { entries, server_id, remaining })
}

pub(crate) fn parse_login(
    payload: &[u8],
    artifact: &RuntimeArtifact,
) -> Result<(i64, i32), ProtocolError> {
    let player_id = required_varint(payload, artifact.protocol.login_player_id_field)?;
    let server_id = required_varint(payload, artifact.protocol.login_server_id_field)?;
    let player_id = i64::from_ne_bytes(player_id.to_ne_bytes());
    let server_id = decode_int32(server_id)?;
    Ok((player_id, server_id))
}

fn validate_mail_entity(
    data: &[u8],
    artifact: &RuntimeArtifact,
) -> Result<Option<i32>, ProtocolError> {
    let mut server_id = None;
    let mut cursor = FieldCursor::new(data);
    while let Some(field) = cursor.next()? {
        if !artifact.protocol.mail_entity.accepts(field.number, field.wire) {
            return Err(ProtocolError::WrongWireType);
        }
        if field.number == artifact.protocol.mail_server_id_field {
            let FieldValue::Varint(value) = field.value else {
                return Err(ProtocolError::WrongWireType);
            };
            server_id = Some(decode_int32(value)?);
        }
    }
    Ok(server_id)
}

fn decode_int32(value: u64) -> Result<i32, ProtocolError> {
    u32::try_from(value)
        .map(|value| i32::from_ne_bytes(value.to_ne_bytes()))
        .map_err(|_error| ProtocolError::IntegerOutOfRange)
}

fn parse_msg<'a>(
    data: &'a [u8],
    artifact: &RuntimeArtifact,
) -> Result<EffectiveMessage<'a>, ProtocolError> {
    let api_id = u32::try_from(required_varint(data, artifact.protocol.msg_api_field)?)
        .map_err(|_error| ProtocolError::IntegerOutOfRange)?;
    let payload = required_bytes(data, artifact.protocol.msg_payload_field)?;
    Ok(EffectiveMessage { api_id, payload: Cow::Borrowed(payload) })
}

fn inflate_wrapper(data: &[u8], schema: CompressionSchema) -> Result<Vec<u8>, ProtocolError> {
    let declared = usize::try_from(required_varint(data, schema.length_field)?)
        .map_err(|_error| ProtocolError::DeclaredInflationTooLarge)?;
    if declared > MAX_INFLATED_BYTES {
        return Err(ProtocolError::DeclaredInflationTooLarge);
    }
    let compressed = required_bytes(data, schema.payload_field)?;
    let decoder = ZlibDecoder::new(compressed);
    let mut limited = decoder.take((MAX_INFLATED_BYTES + 1) as u64);
    let mut inflated = Vec::with_capacity(declared.min(MAX_INFLATED_BYTES));
    limited.read_to_end(&mut inflated).map_err(|_error| ProtocolError::Inflate)?;
    if inflated.len() > MAX_INFLATED_BYTES {
        return Err(ProtocolError::InflationTooLarge);
    }
    if inflated.len() != declared {
        return Err(ProtocolError::InflationLengthMismatch);
    }
    Ok(inflated)
}

fn required_varint(data: &[u8], field_number: u32) -> Result<u64, ProtocolError> {
    let mut cursor = FieldCursor::new(data);
    let mut value = None;
    while let Some(field) = cursor.next()? {
        if field.number == field_number {
            let FieldValue::Varint(current) = field.value else {
                return Err(ProtocolError::WrongWireType);
            };
            value = Some(current);
        }
    }
    value.ok_or(ProtocolError::MissingField)
}

fn required_bytes(data: &[u8], field_number: u32) -> Result<&[u8], ProtocolError> {
    let mut cursor = FieldCursor::new(data);
    let mut value = None;
    while let Some(field) = cursor.next()? {
        if field.number == field_number {
            let FieldValue::LengthDelimited(current) = field.value else {
                return Err(ProtocolError::WrongWireType);
            };
            value = Some(current);
        }
    }
    value.ok_or(ProtocolError::MissingField)
}

#[derive(Debug)]
struct Field<'a> {
    number: u32,
    wire: u8,
    value: FieldValue<'a>,
}

#[derive(Debug)]
enum FieldValue<'a> {
    Varint(u64),
    Fixed64,
    LengthDelimited(&'a [u8]),
    Fixed32,
}

#[derive(Debug)]
struct FieldCursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> FieldCursor<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn next(&mut self) -> Result<Option<Field<'a>>, ProtocolError> {
        if self.position == self.data.len() {
            return Ok(None);
        }
        let tag = self.read_varint()?;
        let number = u32::try_from(tag >> 3).map_err(|_error| ProtocolError::IntegerOutOfRange)?;
        if number == 0 {
            return Err(ProtocolError::ZeroFieldNumber);
        }
        let wire = u8::try_from(tag & 0x07).map_err(|_error| ProtocolError::UnsupportedWireType)?;
        let value = match wire {
            0 => FieldValue::Varint(self.read_varint()?),
            1 => {
                self.advance(8)?;
                FieldValue::Fixed64
            }
            2 => FieldValue::LengthDelimited(self.read_length_delimited()?),
            5 => {
                self.advance(4)?;
                FieldValue::Fixed32
            }
            _ => return Err(ProtocolError::UnsupportedWireType),
        };
        Ok(Some(Field { number, wire, value }))
    }

    fn read_varint(&mut self) -> Result<u64, ProtocolError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self.data.get(self.position).ok_or(ProtocolError::Truncated)?;
            self.position = self.position.saturating_add(1);
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ProtocolError::VarintOverflow)
    }

    fn read_length_delimited(&mut self) -> Result<&'a [u8], ProtocolError> {
        let length = usize::try_from(self.read_varint()?)
            .map_err(|_error| ProtocolError::IntegerOutOfRange)?;
        let end = self.position.checked_add(length).ok_or(ProtocolError::Truncated)?;
        let bytes = self.data.get(self.position..end).ok_or(ProtocolError::Truncated)?;
        self.position = end;
        Ok(bytes)
    }

    fn advance(&mut self, length: usize) -> Result<(), ProtocolError> {
        let end = self.position.checked_add(length).ok_or(ProtocolError::Truncated)?;
        self.data.get(self.position..end).ok_or(ProtocolError::Truncated)?;
        self.position = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_candidates_extracts_nonzero_server_id() {
        let artifact = RuntimeArtifact::test_fixture();
        let entity = [0x0a, 0x01, b'1', 0x68, 0x8c, 0x7d];
        let payload = [vec![0x0a, entity.len() as u8], entity.to_vec()].concat();

        let candidates = mail_candidates(&payload, &artifact, 7909, MAX_MAIL_ENTRIES)
            .expect("mail carrier should parse");

        assert_eq!(candidates.server_id, Some(16_012));
    }

    #[test]
    fn mail_candidates_ignores_zero_server_id_for_context() {
        let artifact = RuntimeArtifact::test_fixture();
        let entity = [0x0a, 0x01, b'1', 0x68, 0x00];
        let payload = [vec![0x0a, entity.len() as u8], entity.to_vec()].concat();

        let candidates = mail_candidates(&payload, &artifact, 7909, MAX_MAIL_ENTRIES)
            .expect("mail carrier should parse");

        assert_eq!(candidates.server_id, None);
    }

    #[test]
    fn mail_candidates_extracts_remaining_page_count() {
        let artifact = RuntimeArtifact::test_fixture();
        let entity = [0x0a, 0x01, b'1'];
        let payload = [vec![0x0a, entity.len() as u8], entity.to_vec(), vec![0x10, 0x0e]].concat();

        let candidates = mail_candidates(&payload, &artifact, 7921, MAX_MAIL_ENTRIES)
            .expect("mail carrier should parse");

        assert_eq!(candidates.remaining, Some(14));
    }

    #[test]
    fn field_cursor_rejects_truncated_length_delimited_value() {
        let mut cursor = FieldCursor::new(&[0x0a, 0x02, 0xaa]);

        let error = cursor.next().expect_err("field should be truncated");

        assert_eq!(error, ProtocolError::Truncated);
    }

    #[test]
    fn field_cursor_skips_unknown_fixed_fields_without_allocating() {
        let mut bytes = vec![0x09];
        bytes.extend_from_slice(&[0; 8]);
        bytes.push(0x15);
        bytes.extend_from_slice(&[0; 4]);
        let mut cursor = FieldCursor::new(&bytes);

        let first = cursor.next().expect("first field should parse");
        let second = cursor.next().expect("second field should parse");
        let end = cursor.next().expect("end should parse");

        assert!(matches!(first, Some(Field { number: 1, value: FieldValue::Fixed64, .. })));
        assert!(matches!(second, Some(Field { number: 2, value: FieldValue::Fixed32, .. })));
        assert!(end.is_none());
    }
}
