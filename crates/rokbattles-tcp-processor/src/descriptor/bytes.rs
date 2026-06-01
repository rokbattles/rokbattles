use std::io::Read as _;

use flate2::read::ZlibDecoder;
use serde_json::{Value, json};

use crate::proto::{RawValue, parse_fields};

pub(super) fn text_or_json_value(bytes: &[u8]) -> Option<Value> {
    if let Ok(text) = std::str::from_utf8(bytes)
        && text.chars().all(|ch| ch == '\n' || ch == '\r' || ch == '\t' || !ch.is_control())
    {
        let trimmed = text.trim();
        if matches!(trimmed.as_bytes().first(), Some(b'{' | b'['))
            && let Ok(value) = serde_json::from_str::<Value>(trimmed)
        {
            return Some(value);
        }
        return Some(Value::String(text.to_string()));
    }
    None
}

pub(super) fn zlib_text_or_json_value(bytes: &[u8]) -> Option<Value> {
    let offset = zlib_offset(bytes)?;
    let mut decoder = ZlibDecoder::new(bytes.get(offset..)?);
    let mut inflated = Vec::new();
    decoder.read_to_end(&mut inflated).ok()?;
    text_or_json_value(&inflated)
}

fn zlib_offset(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|header| {
        let [cmf, flg] = header else {
            return false;
        };
        cmf & 0x0f == 8 && u16::from_be_bytes([*cmf, *flg]) % 31 == 0
    })
}

pub(super) fn protobuf_text_value(bytes: &[u8]) -> Option<Value> {
    let fields = parse_fields(bytes)?;
    single_field_text_value(&fields)
}

fn single_field_text_value(fields: &[crate::proto::RawField]) -> Option<Value> {
    let [field] = fields else {
        return None;
    };
    if field.number != 1 {
        return None;
    }
    let RawValue::LengthDelimited(bytes) = &field.value else {
        return None;
    };
    text_or_json_value(bytes)
}

pub(super) fn compact_bitset_value(bytes: &[u8]) -> Option<Value> {
    let _ = bytes.len().checked_mul(8)?;
    if bytes.len() < 8 {
        return None;
    }

    let mut ranges = Vec::new();
    let mut range_start = None;
    let mut bit_index = 0usize;

    for byte in bytes {
        for bit in 0..8 {
            let is_set = byte & (1 << bit) != 0;
            match (range_start, is_set) {
                (None, true) => range_start = Some(bit_index),
                (Some(start), false) => {
                    ranges.push(json!([start, bit_index - 1]));
                    range_start = None;
                }
                _ => {}
            }
            bit_index += 1;
        }
    }

    if let Some(start) = range_start {
        ranges.push(json!([start, bit_index - 1]));
    }

    if ranges.is_empty() {
        return None;
    }

    let max_ranges = (bytes.len() / 32).clamp(1, 4096);
    if ranges.len() > max_ranges {
        return None;
    }

    Some(Value::Array(ranges))
}
