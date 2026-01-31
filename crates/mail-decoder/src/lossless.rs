//! Lossless decoder implementation and lossless JSON conversion.

use serde_json::{Map, Value};

use crate::common::{
    DecodeError, MAX_DEPTH, TAG_BOOL, TAG_F32, TAG_F64, TAG_OBJECT, TAG_STRING, is_known_tag,
};

/// Lossless decoded document containing any leading preamble bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct LosslessDocument {
    /// Bytes before the decoded payload (header/preamble).
    pub preamble: Vec<u8>,
    /// The decoded payload value.
    pub value: LosslessValue,
}

impl LosslessDocument {
    /// Encode the document back into its original binary representation.
    pub fn to_bytes(&self) -> Result<Vec<u8>, LosslessEncodeError> {
        encode_lossless(self)
    }

    /// Convert the document into a JSON value for inspection or storage.
    pub fn to_json_value(&self) -> Value {
        lossless_to_json(self)
    }
}

/// A lossless representation of a decoded value.
#[derive(Debug, Clone, PartialEq)]
pub enum LosslessValue {
    /// Boolean value stored as the raw `u8` byte.
    Bool {
        /// Raw boolean byte.
        value: u8,
    },
    /// `f32` stored as raw little-endian bytes.
    F32 {
        /// Raw IEEE-754 bytes.
        raw: [u8; 4],
    },
    /// `f64` stored as raw big-endian bytes.
    F64 {
        /// Raw IEEE-754 bytes.
        raw: [u8; 8],
    },
    /// UTF-8 string value.
    String {
        /// String value.
        value: String,
    },
    /// Object or array container.
    Container(LosslessContainer),
    /// Unknown tag preserved verbatim.
    Unknown {
        /// Raw tag byte.
        ///
        /// This stores only the tag byte. If future tags introduce payloads, the
        /// lossless decoder cannot preserve those payload bytes without knowing
        /// their length.
        tag: u8,
    },
}

/// Lossless container value.
#[derive(Debug, Clone, PartialEq)]
pub enum LosslessContainer {
    /// Map-like container with string keys.
    Object(LosslessObject),
    /// Sequential container of values.
    Array(LosslessArray),
}

/// Lossless object container preserving entry order.
#[derive(Debug, Clone, PartialEq)]
pub struct LosslessObject {
    /// Ordered key/value entries.
    pub entries: Vec<LosslessEntry>,
    /// Optional terminator tag consumed at the end of the object.
    pub terminator: Option<u8>,
}

/// Lossless array container preserving item order.
#[derive(Debug, Clone, PartialEq)]
pub struct LosslessArray {
    /// Ordered array items.
    pub items: Vec<LosslessValue>,
    /// Optional terminator tag consumed at the end of the array.
    pub terminator: Option<u8>,
}

/// Lossless object entry.
#[derive(Debug, Clone, PartialEq)]
pub struct LosslessEntry {
    /// String key.
    pub key: String,
    /// Value associated with the key.
    pub value: LosslessValue,
}

/// Errors returned by [encode_lossless].
#[derive(Debug)]
pub enum LosslessEncodeError {
    /// A string length exceeded the maximum supported length.
    StringTooLong {
        /// String length in bytes.
        length: usize,
    },
}

impl std::fmt::Display for LosslessEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LosslessEncodeError::StringTooLong { length } => {
                write!(f, "string length {length} exceeds u32 limit")
            }
        }
    }
}

impl std::error::Error for LosslessEncodeError {}

/// Decode a binary mail buffer into a lossless document.
///
/// The decoder returns the payload along with any leading preamble bytes. If the
/// payload does not consume all trailing bytes and the first tag is not unknown,
/// the function reports trailing bytes as an error.
pub fn decode_lossless(buffer: &[u8]) -> Result<LosslessDocument, DecodeError> {
    if buffer.is_empty() {
        return Err(DecodeError::UnexpectedEof {
            needed: 1,
            remaining: 0,
        });
    }

    let mut decoder = LosslessDecoder::new(buffer);
    let value = decoder.read_value()?;
    if decoder.remaining() == 0 {
        return Ok(LosslessDocument {
            preamble: Vec::new(),
            value,
        });
    }

    if !matches!(value, LosslessValue::Unknown { .. }) {
        return Err(DecodeError::TrailingBytes {
            remaining: decoder.remaining(),
        });
    }

    let remaining = decoder.remaining();
    if let Some((offset, value)) = find_lossless_payload(buffer) {
        return Ok(LosslessDocument {
            preamble: buffer[..offset].to_vec(),
            value,
        });
    }

    Err(DecodeError::TrailingBytes { remaining })
}

/// Encode a lossless document back into bytes.
pub fn encode_lossless(document: &LosslessDocument) -> Result<Vec<u8>, LosslessEncodeError> {
    let mut buffer = Vec::with_capacity(document.preamble.len() + 32);
    buffer.extend_from_slice(&document.preamble);
    write_lossless_value(&document.value, &mut buffer)?;
    Ok(buffer)
}

/// Convert a lossless document into JSON for storage or inspection.
pub fn lossless_to_json(document: &LosslessDocument) -> Value {
    let mut root = Map::new();
    root.insert(
        "preamble_hex".to_string(),
        Value::String(hex_encode(&document.preamble)),
    );
    root.insert("value".to_string(), lossless_value_to_json(&document.value));
    Value::Object(root)
}

fn lossless_value_to_json(value: &LosslessValue) -> Value {
    let mut object = Map::new();
    match value {
        LosslessValue::Bool { value } => {
            object.insert("tag".to_string(), Value::String("bool".to_string()));
            object.insert("raw".to_string(), Value::from(*value));
        }
        LosslessValue::F32 { raw } => {
            object.insert("tag".to_string(), Value::String("f32_le".to_string()));
            object.insert("raw_hex".to_string(), Value::String(hex_encode(raw)));
        }
        LosslessValue::F64 { raw } => {
            object.insert("tag".to_string(), Value::String("f64_be".to_string()));
            object.insert("raw_hex".to_string(), Value::String(hex_encode(raw)));
        }
        LosslessValue::String { value } => {
            object.insert("tag".to_string(), Value::String("string".to_string()));
            object.insert("value".to_string(), Value::String(value.clone()));
        }
        LosslessValue::Container(container) => {
            object.insert("tag".to_string(), Value::String("container".to_string()));
            match container {
                LosslessContainer::Object(obj) => {
                    object.insert("kind".to_string(), Value::String("object".to_string()));
                    let entries = obj
                        .entries
                        .iter()
                        .map(|entry| {
                            let mut entry_obj = Map::new();
                            entry_obj.insert("key".to_string(), Value::String(entry.key.clone()));
                            entry_obj
                                .insert("value".to_string(), lossless_value_to_json(&entry.value));
                            Value::Object(entry_obj)
                        })
                        .collect::<Vec<_>>();
                    object.insert("entries".to_string(), Value::Array(entries));
                    if let Some(tag) = obj.terminator {
                        object.insert("terminator".to_string(), Value::from(tag));
                    }
                }
                LosslessContainer::Array(array) => {
                    object.insert("kind".to_string(), Value::String("array".to_string()));
                    let items = array
                        .items
                        .iter()
                        .map(lossless_value_to_json)
                        .collect::<Vec<_>>();
                    object.insert("items".to_string(), Value::Array(items));
                    if let Some(tag) = array.terminator {
                        object.insert("terminator".to_string(), Value::from(tag));
                    }
                }
            }
        }
        LosslessValue::Unknown { tag } => {
            object.insert("tag".to_string(), Value::String("unknown".to_string()));
            object.insert("raw".to_string(), Value::from(*tag));
        }
    }

    Value::Object(object)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + (nibble - 10)) as char,
    }
}

fn find_lossless_payload(buffer: &[u8]) -> Option<(usize, LosslessValue)> {
    let mut fallback = None;
    for (offset, tag) in buffer.iter().enumerate() {
        if !is_known_tag(*tag) {
            continue;
        }

        let mut decoder = LosslessDecoder::with_offset(buffer, offset);
        if let Ok(value) = decoder.read_value()
            && decoder.remaining() == 0
        {
            if matches!(&value, LosslessValue::Container(_)) {
                return Some((offset, value));
            }
            if fallback.is_none() {
                fallback = Some((offset, value));
            }
        }
    }

    fallback
}

struct LosslessDecoder<'a> {
    buffer: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> LosslessDecoder<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            pos: 0,
            depth: 0,
        }
    }

    fn with_offset(buffer: &'a [u8], pos: usize) -> Self {
        Self {
            buffer,
            pos,
            depth: 0,
        }
    }

    fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    fn read_value(&mut self) -> Result<LosslessValue, DecodeError> {
        let tag = self.read_u8()?;
        match tag {
            TAG_BOOL => Ok(LosslessValue::Bool {
                value: self.read_u8()?,
            }),
            TAG_F32 => {
                let raw = self.read_exact(4)?;
                Ok(LosslessValue::F32 {
                    raw: raw.try_into().expect("slice length checked"),
                })
            }
            TAG_F64 => {
                let raw = self.read_exact(8)?;
                Ok(LosslessValue::F64 {
                    raw: raw.try_into().expect("slice length checked"),
                })
            }
            TAG_STRING => {
                let value = self.read_string()?;
                Ok(LosslessValue::String { value })
            }
            TAG_OBJECT => self.read_container(),
            _ => Ok(LosslessValue::Unknown { tag }),
        }
    }

    fn read_container(&mut self) -> Result<LosslessValue, DecodeError> {
        if self.depth >= MAX_DEPTH {
            return Err(DecodeError::DepthLimitExceeded { limit: MAX_DEPTH });
        }

        self.depth += 1;
        let container = match self.peek_u8() {
            Some(TAG_STRING) => LosslessContainer::Object(self.read_object_entries()?),
            Some(_) => LosslessContainer::Array(self.read_array_entries()?),
            None => LosslessContainer::Object(LosslessObject {
                entries: Vec::new(),
                terminator: None,
            }),
        };
        self.depth -= 1;

        Ok(LosslessValue::Container(container))
    }

    fn read_object_entries(&mut self) -> Result<LosslessObject, DecodeError> {
        let mut entries = Vec::new();
        let mut terminator = None;

        while let Some(tag) = self.peek_u8() {
            if tag == TAG_STRING {
                let _ = self.read_u8()?;
                let key = self.read_string()?;
                let value = self.read_value()?;
                entries.push(LosslessEntry { key, value });
                continue;
            }

            if !is_known_tag(tag) {
                terminator = Some(self.read_u8()?);
            }
            break;
        }

        Ok(LosslessObject {
            entries,
            terminator,
        })
    }

    fn read_array_entries(&mut self) -> Result<LosslessArray, DecodeError> {
        let mut items = Vec::new();
        let mut terminator = None;

        while let Some(tag) = self.peek_u8() {
            if !is_known_tag(tag) {
                terminator = Some(self.read_u8()?);
                break;
            }

            let value = self.read_value()?;
            items.push(value);
        }

        Ok(LosslessArray { items, terminator })
    }

    fn read_string(&mut self) -> Result<String, DecodeError> {
        let length = self.read_u32_le()? as usize;
        let remaining = self.remaining();
        if length > remaining {
            return Err(DecodeError::LengthOutOfBounds { length, remaining });
        }

        let start = self.pos;
        let bytes = self.read_exact(length)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| DecodeError::InvalidUtf8 { offset: start })
    }

    fn read_u32_le(&mut self) -> Result<u32, DecodeError> {
        let raw = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            raw.try_into().expect("slice length checked"),
        ))
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        let byte = self
            .buffer
            .get(self.pos)
            .copied()
            .ok_or(DecodeError::UnexpectedEof {
                needed: 1,
                remaining: 0,
            })?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.saturating_add(len);
        if end > self.buffer.len() {
            return Err(DecodeError::UnexpectedEof {
                needed: len,
                remaining: self.remaining(),
            });
        }

        let start = self.pos;
        self.pos = end;
        Ok(&self.buffer[start..end])
    }

    fn peek_u8(&self) -> Option<u8> {
        self.buffer.get(self.pos).copied()
    }
}

fn write_lossless_value(
    value: &LosslessValue,
    buffer: &mut Vec<u8>,
) -> Result<(), LosslessEncodeError> {
    match value {
        LosslessValue::Bool { value } => {
            buffer.push(TAG_BOOL);
            buffer.push(*value);
        }
        LosslessValue::F32 { raw } => {
            buffer.push(TAG_F32);
            buffer.extend_from_slice(raw);
        }
        LosslessValue::F64 { raw } => {
            buffer.push(TAG_F64);
            buffer.extend_from_slice(raw);
        }
        LosslessValue::String { value } => {
            write_string_tagged(value, buffer)?;
        }
        LosslessValue::Container(container) => {
            buffer.push(TAG_OBJECT);
            match container {
                LosslessContainer::Object(object) => {
                    for entry in &object.entries {
                        write_string_tagged(&entry.key, buffer)?;
                        write_lossless_value(&entry.value, buffer)?;
                    }
                    if let Some(tag) = object.terminator {
                        buffer.push(tag);
                    }
                }
                LosslessContainer::Array(array) => {
                    for item in &array.items {
                        write_lossless_value(item, buffer)?;
                    }
                    if let Some(tag) = array.terminator {
                        buffer.push(tag);
                    }
                }
            }
        }
        LosslessValue::Unknown { tag } => {
            buffer.push(*tag);
        }
    }

    Ok(())
}

fn write_string_tagged(value: &str, buffer: &mut Vec<u8>) -> Result<(), LosslessEncodeError> {
    buffer.push(TAG_STRING);
    write_string_bytes(value.as_bytes(), buffer)
}

fn write_string_bytes(bytes: &[u8], buffer: &mut Vec<u8>) -> Result<(), LosslessEncodeError> {
    let length = u32::try_from(bytes.len()).map_err(|_| LosslessEncodeError::StringTooLong {
        length: bytes.len(),
    })?;
    buffer.extend_from_slice(&length.to_le_bytes());
    buffer.extend_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_OBJECT_END: u8 = 0xff;

    fn encode_string(value: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(TAG_STRING);
        buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buffer.extend_from_slice(value.as_bytes());
        buffer
    }

    fn encode_object(pairs: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut buffer = vec![TAG_OBJECT];
        for (key, value) in pairs {
            buffer.extend_from_slice(&encode_string(key));
            buffer.extend_from_slice(value);
        }
        buffer.push(TAG_OBJECT_END);
        buffer
    }

    fn encode_array(values: &[Vec<u8>]) -> Vec<u8> {
        let mut buffer = vec![TAG_OBJECT];
        for value in values {
            buffer.extend_from_slice(value);
        }
        buffer.push(TAG_OBJECT_END);
        buffer
    }

    #[test]
    fn decode_lossless_bool_preserves_raw() {
        let doc = decode_lossless(&[TAG_BOOL, 2]).unwrap();
        assert!(doc.preamble.is_empty());
        assert_eq!(doc.value, LosslessValue::Bool { value: 2 });
    }

    #[test]
    fn decode_lossless_object_entries_preserve_order() {
        let buffer = encode_object(&[("a", vec![TAG_BOOL, 1]), ("b", encode_string("ok"))]);
        let doc = decode_lossless(&buffer).unwrap();
        let LosslessValue::Container(LosslessContainer::Object(object)) = doc.value else {
            panic!("expected object container");
        };

        assert_eq!(object.entries.len(), 2);
        assert_eq!(object.entries[0].key, "a");
        assert_eq!(object.entries[0].value, LosslessValue::Bool { value: 1 });
        assert_eq!(object.entries[1].key, "b");
        assert_eq!(
            object.entries[1].value,
            LosslessValue::String {
                value: "ok".to_string()
            }
        );
        assert_eq!(object.terminator, Some(TAG_OBJECT_END));
    }

    #[test]
    fn decode_lossless_array_items_preserve_order() {
        let buffer = encode_array(&[vec![TAG_BOOL, 1], encode_string("ok")]);
        let doc = decode_lossless(&buffer).unwrap();
        let LosslessValue::Container(LosslessContainer::Array(ref array)) = doc.value else {
            panic!("expected array container");
        };

        assert_eq!(array.items.len(), 2);
        assert_eq!(array.items[0], LosslessValue::Bool { value: 1 });
        assert_eq!(
            array.items[1],
            LosslessValue::String {
                value: "ok".to_string()
            }
        );
        assert_eq!(array.terminator, Some(TAG_OBJECT_END));
    }

    #[test]
    fn lossless_round_trip_small_buffer() {
        let mut buffer = vec![0xff, 0x00, 0x10];
        buffer.extend_from_slice(&encode_array(&[vec![TAG_BOOL, 1]]));

        let doc = decode_lossless(&buffer).unwrap();
        assert_eq!(doc.preamble, vec![0xff, 0x00, 0x10]);
        let encoded = encode_lossless(&doc).unwrap();
        assert_eq!(encoded, buffer);
    }

    #[test]
    fn lossless_round_trip_unknown_tag_value() {
        let buffer = vec![0x99];
        let doc = decode_lossless(&buffer).unwrap();
        assert_eq!(doc.preamble, Vec::<u8>::new());
        assert_eq!(doc.value, LosslessValue::Unknown { tag: 0x99 });

        let encoded = encode_lossless(&doc).unwrap();
        assert_eq!(encoded, buffer);
    }

    #[test]
    fn lossless_preserves_unknown_container_terminator() {
        let buffer = vec![TAG_OBJECT, TAG_BOOL, 1, 0xee];
        let doc = decode_lossless(&buffer).unwrap();
        let LosslessValue::Container(LosslessContainer::Array(ref array)) = doc.value else {
            panic!("expected array container");
        };

        assert_eq!(array.items.len(), 1);
        assert_eq!(array.items[0], LosslessValue::Bool { value: 1 });
        assert_eq!(array.terminator, Some(0xee));

        let encoded = encode_lossless(&doc).unwrap();
        assert_eq!(encoded, buffer);
    }

    #[test]
    fn lossless_json_includes_preamble_hex() {
        let doc = LosslessDocument {
            preamble: vec![0xaa, 0xbb],
            value: LosslessValue::Bool { value: 1 },
        };

        let json = lossless_to_json(&doc);
        assert_eq!(json["preamble_hex"], Value::String("aabb".to_string()));
    }

    #[test]
    fn lossless_round_trip_sample_mail_files() {
        let samples = [
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../samples/Battle/Persistent.Mail.485440176891031331"
            ))
            .as_slice(),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../samples/Battle/Persistent.Mail.1409019176893142331"
            ))
            .as_slice(),
        ];

        for sample in samples {
            let doc = decode_lossless(sample).expect("decode lossless");
            let encoded = encode_lossless(&doc).expect("encode lossless");
            assert_eq!(encoded, sample);
        }
    }
}
