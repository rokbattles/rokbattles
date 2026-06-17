//! Byte cursor used by the decoder.

use serde_json::{Map, Value};

use crate::{
    error::DecodeError,
    format::{
        MAX_DEPTH, MAX_PREAMBLE_SCAN_BYTES, TAG_BOOL, TAG_CONTAINER, TAG_F32, TAG_F64, TAG_STRING,
        is_known_tag,
    },
    number::number_value,
    value::numeric_keyed_container,
};

/// Decode a persistent mail buffer into JSON.
pub fn decode(buffer: &[u8]) -> Result<Value, DecodeError> {
    if buffer.is_empty() {
        return Err(DecodeError::UnexpectedEof { needed: 1, remaining: 0 });
    }

    let mut cursor = Cursor::new(buffer);
    let value = cursor.read_value()?;
    if cursor.remaining() == 0 {
        return Ok(value);
    }

    if !matches!(value, Value::Null) {
        return Err(DecodeError::TrailingBytes { remaining: cursor.remaining() });
    }

    let remaining = cursor.remaining();
    find_payload_value(buffer).ok_or(DecodeError::TrailingBytes { remaining })
}

fn find_payload_value(buffer: &[u8]) -> Option<Value> {
    let mut fallback = None;
    for (offset, tag) in buffer.iter().copied().enumerate() {
        if offset > MAX_PREAMBLE_SCAN_BYTES {
            break;
        }
        if !is_known_tag(tag) {
            continue;
        }

        // Mail files have a short header before the tagged payload. Try each
        // plausible tag in that header window and keep only full-buffer decodes.
        let mut cursor = Cursor::with_offset(buffer, offset);
        if let Ok(value) = cursor.read_value()
            && cursor.remaining() == 0
        {
            if matches!(&value, Value::Object(_) | Value::Array(_)) {
                return Some(value);
            }
            if fallback.is_none() {
                fallback = Some(value);
            }
        }
    }

    fallback
}

struct Cursor<'a> {
    buffer: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Cursor<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, pos: 0, depth: 0 }
    }

    fn with_offset(buffer: &'a [u8], pos: usize) -> Self {
        Self { buffer, pos, depth: 0 }
    }

    fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    fn read_value(&mut self) -> Result<Value, DecodeError> {
        let tag = self.read_u8()?;
        match tag {
            TAG_BOOL => Ok(Value::Bool(self.read_u8()? != 0)),
            TAG_F32 => {
                let raw = self.read_exact(4)?;
                let bytes = bytes4(raw)?;
                number_value(f64::from(f32::from_le_bytes(bytes)))
            }
            TAG_F64 => {
                let raw = self.read_exact(8)?;
                let bytes = bytes8(raw)?;
                number_value(f64::from_be_bytes(bytes))
            }
            TAG_STRING => Ok(Value::String(self.read_string()?)),
            TAG_CONTAINER => self.read_container(),
            _ => Ok(Value::Null),
        }
    }

    fn read_container(&mut self) -> Result<Value, DecodeError> {
        if self.depth >= MAX_DEPTH {
            return Err(DecodeError::DepthLimitExceeded { limit: MAX_DEPTH });
        }

        self.depth += 1;
        // The container tag does not tell us whether this is a list or a map.
        // A string child starts a normal object; numeric children need one pass
        // through `numeric_keyed_container` after the container has been read.
        let value = match self.peek_u8() {
            Some(TAG_STRING) => Value::Object(self.read_string_keyed_object()?),
            Some(_) => self.read_numeric_or_sequential_container()?,
            // The old lossless decoder used `{}` for a bare container at EOF.
            // Keep that edge case so migration does not reshape it.
            None => Value::Object(Map::new()),
        };
        self.depth -= 1;
        Ok(value)
    }

    fn read_string_keyed_object(&mut self) -> Result<Map<String, Value>, DecodeError> {
        let mut map = Map::new();

        while let Some(tag) = self.peek_u8() {
            if tag == TAG_STRING {
                let _tag = self.read_u8()?;
                let key = self.read_string()?;
                let value = self.read_value()?;
                map.insert(key, value);
                continue;
            }

            // Unknown tags are used as explicit container terminators. Known
            // non-string tags belong to the caller, so leave them unread.
            if !is_known_tag(tag) {
                let _terminator = self.read_u8()?;
            }
            break;
        }

        Ok(map)
    }

    fn read_numeric_or_sequential_container(&mut self) -> Result<Value, DecodeError> {
        let mut items = Vec::new();

        while let Some(tag) = self.peek_u8() {
            if !is_known_tag(tag) {
                let _terminator = self.read_u8()?;
                break;
            }

            items.push(self.read_value()?);
        }

        match numeric_keyed_container(&items) {
            Some(container) => Ok(container),
            None => Ok(Value::Array(items)),
        }
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
        Ok(u32::from_le_bytes(bytes4(raw)?))
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        let byte = self
            .buffer
            .get(self.pos)
            .copied()
            .ok_or(DecodeError::UnexpectedEof { needed: 1, remaining: 0 })?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.saturating_add(len);
        if end > self.buffer.len() {
            return Err(DecodeError::UnexpectedEof { needed: len, remaining: self.remaining() });
        }

        let start = self.pos;
        self.pos = end;
        Ok(&self.buffer[start..end])
    }

    fn peek_u8(&self) -> Option<u8> {
        self.buffer.get(self.pos).copied()
    }
}

fn bytes4(bytes: &[u8]) -> Result<[u8; 4], DecodeError> {
    <[u8; 4]>::try_from(bytes)
        .map_err(|_| DecodeError::UnexpectedEof { needed: 4, remaining: bytes.len() })
}

fn bytes8(bytes: &[u8]) -> Result<[u8; 8], DecodeError> {
    <[u8; 8]>::try_from(bytes)
        .map_err(|_| DecodeError::UnexpectedEof { needed: 8, remaining: bytes.len() })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const TAG_CONTAINER_END: u8 = 0xff;

    fn decode_test(buffer: &[u8]) -> Value {
        match decode(buffer) {
            Ok(value) => value,
            Err(error) => panic!("decode failed: {error}"),
        }
    }

    fn encode_f64(value: f64) -> Vec<u8> {
        let mut buffer = vec![TAG_F64];
        buffer.extend_from_slice(&value.to_be_bytes());
        buffer
    }

    fn encode_string(value: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(TAG_STRING);
        buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buffer.extend_from_slice(value.as_bytes());
        buffer
    }

    fn encode_container(values: &[Vec<u8>]) -> Vec<u8> {
        let mut buffer = vec![TAG_CONTAINER];
        for value in values {
            buffer.extend_from_slice(value);
        }
        buffer.push(TAG_CONTAINER_END);
        buffer
    }

    #[test]
    fn decode_bare_container_at_eof_as_empty_object() {
        let decoded = decode_test(&[TAG_CONTAINER]);

        assert_eq!(decoded, json!({}));
    }

    #[test]
    fn decode_terminated_empty_container_as_empty_array() {
        let decoded = decode_test(&[TAG_CONTAINER, TAG_CONTAINER_END]);

        assert_eq!(decoded, json!([]));
    }

    #[test]
    fn decode_sequential_numeric_keyed_container_as_array() {
        let buffer = encode_container(&[
            encode_f64(1.0),
            encode_string("first"),
            encode_f64(2.0),
            encode_string("second"),
        ]);

        let decoded = decode_test(&buffer);

        assert_eq!(decoded, json!(["first", "second"]));
    }

    #[test]
    fn decode_non_consecutive_numeric_keyed_container_as_object() {
        let buffer = encode_container(&[encode_f64(2.0), encode_string("value")]);

        let decoded = decode_test(&buffer);

        assert_eq!(decoded, json!({"2": "value"}));
    }

    #[test]
    fn decode_sequential_numeric_scalar_container_as_array() {
        let buffer = encode_container(&[
            encode_f64(1.0),
            encode_f64(24.0),
            encode_f64(2.0),
            encode_f64(1.0),
        ]);

        let decoded = decode_test(&buffer);

        assert_eq!(decoded, json!([24, 1]));
    }

    #[test]
    fn decode_string_keyed_container_as_object() {
        let buffer = encode_container(&[encode_string("name"), encode_string("value")]);

        let decoded = decode_test(&buffer);

        assert_eq!(decoded, json!({"name": "value"}));
    }

    #[test]
    fn decode_buffer_with_observed_preamble_margin() {
        let payload = encode_container(&[encode_f64(1.0), encode_string("first")]);
        let mut buffer = vec![0xaa; MAX_PREAMBLE_SCAN_BYTES];
        buffer.extend_from_slice(&payload);

        let decoded = decode_test(&buffer);

        assert_eq!(decoded, json!(["first"]));
    }

    #[test]
    fn decode_rejects_preamble_beyond_scan_limit() {
        let payload = encode_container(&[encode_f64(1.0), encode_string("first")]);
        let mut buffer = vec![0xaa; MAX_PREAMBLE_SCAN_BYTES + 1];
        buffer.extend_from_slice(&payload);

        let error = match decode(&buffer) {
            Ok(value) => panic!("expected error, got {value:?}"),
            Err(error) => error,
        };

        assert!(matches!(error, DecodeError::TrailingBytes { .. }));
    }

    #[test]
    fn decode_checked_in_mail_samples() {
        let samples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
        let mut decoded_files = 0;

        for input in collect_mail_samples(&samples_dir) {
            let buffer = std::fs::read(&input).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", input.display());
            });
            if let Err(error) = decode(&buffer) {
                panic!("failed to decode {}: {error}", input.display());
            }
            decoded_files += 1;
        }

        assert_eq!(decoded_files, 119);
    }

    fn collect_mail_samples(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut samples = Vec::new();
        collect_mail_samples_inner(dir, &mut samples);
        samples.sort();
        samples
    }

    fn collect_mail_samples_inner(dir: &std::path::Path, samples: &mut Vec<std::path::PathBuf>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", dir.display());
        });

        for entry in entries {
            let path = entry
                .unwrap_or_else(|error| {
                    panic!("failed to read entry in {}: {error}", dir.display())
                })
                .path();
            if path.is_dir() {
                collect_mail_samples_inner(&path, samples);
            } else if is_binary_mail_sample(&path) {
                samples.push(path);
            }
        }
    }

    fn is_binary_mail_sample(path: &std::path::Path) -> bool {
        let Some(file_name) = path.file_name().and_then(std::ffi::OsStr::to_str) else {
            return false;
        };
        file_name.starts_with("Persistent.Mail.")
            && !file_name.ends_with(".json")
            && !file_name.ends_with("-processed.json")
    }
}
