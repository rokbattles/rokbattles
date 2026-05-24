//! Small protobuf reader and packet wrapper unwrapping helpers.

use std::io::Read;

use flate2::read::ZlibDecoder;

#[derive(Debug, Clone, PartialEq)]
pub enum RawValue {
    Varint(u64),
    Fixed64([u8; 8]),
    LengthDelimited(Vec<u8>),
    Fixed32([u8; 4]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawField {
    pub number: u32,
    pub wire: u8,
    pub value: RawValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgWrapper {
    pub api_id: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedWrapper {
    pub declared_len: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrappedPayload {
    pub api_id: Option<u32>,
    pub payload: Vec<u8>,
}

pub const OUTER_WRAPPER_APIS: &[u32] = &[9999, 61438];

pub fn parse_fields(data: &[u8]) -> Option<Vec<RawField>> {
    let mut cursor = ProtoCursor::new(data);
    let mut fields = Vec::new();
    while !cursor.is_done() {
        let tag = cursor.read_varint()?;
        let number = u32::try_from(tag >> 3).ok()?;
        let wire = u8::try_from(tag & 0x07).ok()?;
        if number == 0 {
            return None;
        }
        let value = match wire {
            0 => RawValue::Varint(cursor.read_varint()?),
            1 => RawValue::Fixed64(cursor.read_array()?),
            2 => RawValue::LengthDelimited(cursor.read_len_delimited()?.to_vec()),
            5 => RawValue::Fixed32(cursor.read_array()?),
            _ => return None,
        };
        fields.push(RawField { number, wire, value });
    }
    Some(fields)
}

pub fn protobuf_value(data: &[u8], field_no: u32) -> Option<RawValue> {
    parse_fields(data)?.into_iter().find(|field| field.number == field_no).map(|field| field.value)
}

pub fn parse_msg_wrapper(data: &[u8]) -> Option<MsgWrapper> {
    let api = protobuf_value(data, 1)?;
    let payload = protobuf_value(data, 2)?;
    match (api, payload) {
        (RawValue::Varint(api_id), RawValue::LengthDelimited(payload)) => {
            Some(MsgWrapper { api_id: u32::try_from(api_id).ok()?, payload })
        }
        _ => None,
    }
}

pub fn parse_compressed_wrapper(data: &[u8]) -> Option<CompressedWrapper> {
    let declared_len = protobuf_value(data, 1)?;
    let payload = protobuf_value(data, 2)?;
    match (declared_len, payload) {
        (RawValue::Varint(declared_len), RawValue::LengthDelimited(payload)) => {
            Some(CompressedWrapper { declared_len, payload })
        }
        _ => None,
    }
}

pub fn unwrap_effective_payload(data: &[u8]) -> Result<UnwrappedPayload, String> {
    let Some(outer) = parse_msg_wrapper(data) else {
        return Ok(UnwrappedPayload { api_id: None, payload: data.to_vec() });
    };

    if !OUTER_WRAPPER_APIS.contains(&outer.api_id) {
        return Ok(UnwrappedPayload { api_id: Some(outer.api_id), payload: outer.payload });
    }

    let Some(compressed) = parse_compressed_wrapper(&outer.payload) else {
        return Ok(UnwrappedPayload { api_id: Some(outer.api_id), payload: outer.payload });
    };
    let inflated = inflate_first_zlib(&compressed.payload)
        .ok_or_else(|| "compressed wrapper did not contain zlib payload".to_string())?;

    if let Some(inner) = parse_msg_wrapper(&inflated) {
        return Ok(UnwrappedPayload { api_id: Some(inner.api_id), payload: inner.payload });
    }

    if let Some(RawValue::LengthDelimited(report_data)) = protobuf_value(&inflated, 1)
        && let Some(inner) = parse_msg_wrapper(&report_data)
    {
        return Ok(UnwrappedPayload { api_id: Some(inner.api_id), payload: inner.payload });
    }

    Ok(UnwrappedPayload { api_id: None, payload: inflated })
}

fn inflate_first_zlib(data: &[u8]) -> Option<Vec<u8>> {
    for index in 0..data.len().saturating_sub(1) {
        if data.get(index) != Some(&0x78) {
            continue;
        }
        if !matches!(data.get(index + 1), Some(0x01 | 0x5e | 0x9c | 0xda)) {
            continue;
        }
        let Some(chunk) = data.get(index..) else {
            continue;
        };
        let mut decoder = ZlibDecoder::new(chunk);
        let mut out = Vec::new();
        if decoder.read_to_end(&mut out).is_ok() {
            return Some(out);
        }
    }
    None
}

pub fn zigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

#[derive(Debug)]
struct ProtoCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ProtoCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_done(&self) -> bool {
        self.pos == self.data.len()
    }

    fn read_varint(&mut self) -> Option<u64> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self.data.get(self.pos)?;
            self.pos += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte < 0x80 {
                return Some(value);
            }
        }
        None
    }

    fn read_len_delimited(&mut self) -> Option<&'a [u8]> {
        let len = usize::try_from(self.read_varint()?).ok()?;
        let end = self.pos.checked_add(len)?;
        let bytes = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(bytes)
    }

    fn read_array<const N: usize>(&mut self) -> Option<[u8; N]> {
        let end = self.pos.checked_add(N)?;
        let bytes = self.data.get(self.pos..end)?;
        self.pos = end;
        bytes.try_into().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_msg_wrapper_reads_api_and_payload() {
        let msg =
            parse_msg_wrapper(&[0x08, 0x0e, 0x12, 0x02, 0xaa, 0xbb]).expect("wrapper should parse");

        assert_eq!(msg.api_id, 14);
        assert_eq!(msg.payload, vec![0xaa, 0xbb]);
    }

    #[test]
    fn unwrap_effective_payload_returns_plain_msg_payload() {
        let unwrapped = unwrap_effective_payload(&[0x08, 0x0e, 0x12, 0x02, 0xaa, 0xbb]).unwrap();

        assert_eq!(unwrapped.api_id, Some(14));
        assert_eq!(unwrapped.payload, vec![0xaa, 0xbb]);
    }
}
