//! Normalized decoder implementation.

use serde_json::{Map, Number, Value};

use crate::{
    common::{
        DecodeError, MAX_DEPTH, TAG_BOOL, TAG_F32, TAG_F64, TAG_OBJECT, TAG_STRING, is_known_tag,
    },
    value::numeric_keyed_container,
};

/// Largest header prefix scanned before the payload.
const MAX_PREAMBLE_SCAN_BYTES: usize = 16;

/// Decode a binary mail buffer into a JSON value.
///
/// The decoder expects a single root value. If extra bytes remain after decoding,
/// an error is returned. When the first parsed value is `null` and trailing bytes
/// exist, the decoder assumes a leading preamble and retries decoding from a
/// limited number of candidate offsets that yield a full, trailing-free decode.
pub fn decode(buffer: &[u8]) -> Result<Value, DecodeError> {
    if buffer.is_empty() {
        return Err(DecodeError::UnexpectedEof { needed: 1, remaining: 0 });
    }

    let mut decoder = Decoder::new(buffer);
    let value = decoder.read_value()?;
    if decoder.remaining() == 0 {
        return Ok(value);
    }

    if !matches!(value, Value::Null) {
        return Err(DecodeError::TrailingBytes { remaining: decoder.remaining() });
    }

    let remaining = decoder.remaining();
    if let Some(value) = find_payload_value(buffer) {
        return Ok(value);
    }

    Err(DecodeError::TrailingBytes { remaining })
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

        let mut decoder = Decoder::with_offset(buffer, offset);
        if let Ok(value) = decoder.read_value()
            && decoder.remaining() == 0
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

struct Decoder<'a> {
    buffer: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Decoder<'a> {
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
            TAG_BOOL => {
                let value = self.read_u8()? != 0;
                Ok(Value::Bool(value))
            }
            TAG_F32 => {
                let raw = self.read_exact(4)?;
                let value = f32::from_le_bytes(bytes4(raw)?);
                number_value(f64::from(value))
            }
            TAG_F64 => {
                let raw = self.read_exact(8)?;
                let value = f64::from_be_bytes(bytes8(raw)?);
                number_value(value)
            }
            TAG_STRING => {
                let value = self.read_string()?;
                Ok(Value::String(value))
            }
            TAG_OBJECT => self.read_container(),
            _ => Ok(Value::Null),
        }
    }

    fn read_container(&mut self) -> Result<Value, DecodeError> {
        if self.depth >= MAX_DEPTH {
            return Err(DecodeError::DepthLimitExceeded { limit: MAX_DEPTH });
        }

        self.depth += 1;
        // The container tag does not tell us whether this is a list or a map.
        // A string child starts a normal object; numeric children are normalized
        // after the complete container has been read.
        let value = match self.peek_u8() {
            Some(TAG_STRING) => Value::Object(self.read_object_entries()?),
            Some(_) => self.read_numeric_or_sequential_container()?,
            None => Value::Object(Map::new()),
        };
        self.depth -= 1;
        Ok(value)
    }

    fn read_object_entries(&mut self) -> Result<Map<String, Value>, DecodeError> {
        let mut map = Map::new();

        while let Some(tag) = self.peek_u8() {
            if tag == TAG_STRING {
                let _ = self.read_u8()?;
                let key = self.read_string()?;
                let value = self.read_value()?;
                map.insert(key, value);
                continue;
            }

            // Non-string tags end the object; unknown tags act as explicit terminators.
            if !is_known_tag(tag) {
                let _ = self.read_u8()?;
            }
            break;
        }

        Ok(map)
    }

    fn read_numeric_or_sequential_container(&mut self) -> Result<Value, DecodeError> {
        let mut items = Vec::new();

        while let Some(tag) = self.peek_u8() {
            if !is_known_tag(tag) {
                let _ = self.read_u8()?;
                break;
            }

            let value = self.read_value()?;
            items.push(value);
        }

        Ok(numeric_keyed_container(&items).unwrap_or(Value::Array(items)))
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

fn number_value(value: f64) -> Result<Value, DecodeError> {
    if value.is_finite() {
        let normalized = normalize_integer(value);
        Ok(Value::Number(normalized))
    } else {
        Err(DecodeError::NonFiniteNumber { value })
    }
}

fn normalize_integer(value: f64) -> Number {
    if value == 0.0 {
        return Number::from(0);
    }

    if value.fract() == 0.0 {
        if value.is_sign_positive() {
            if let Some(int) = to_u64_exact(value) {
                return Number::from(int);
            }
        } else if let Some(int) = to_i64_exact(value) {
            return Number::from(int);
        }
    }

    number_from_f64(value)
}

fn number_from_f64(value: f64) -> Number {
    if let Some(number) = Number::from_f64(value) {
        number
    } else {
        // `number_value` rejects NaN and infinity before this helper is called.
        Number::from(0)
    }
}

fn to_u64_exact(value: f64) -> Option<u64> {
    if value < 0.0 || value > u64::MAX as f64 {
        return None;
    }
    let int = value as u64;
    if (int as f64) == value { Some(int) } else { None }
}

fn to_i64_exact(value: f64) -> Option<i64> {
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return None;
    }
    let int = value as i64;
    if (int as f64) == value { Some(int) } else { None }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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

    fn encode_f64(value: f64) -> Vec<u8> {
        let mut buffer = vec![TAG_F64];
        buffer.extend_from_slice(&value.to_be_bytes());
        buffer
    }

    #[test]
    fn decode_bool_values() {
        let value = decode(&[TAG_BOOL, 1]).unwrap();
        assert_eq!(value, Value::Bool(true));

        let value = decode(&[TAG_BOOL, 0]).unwrap();
        assert_eq!(value, Value::Bool(false));
    }

    #[test]
    fn decode_f32_value() {
        let mut buffer = vec![TAG_F32];
        buffer.extend_from_slice(&1.5_f32.to_le_bytes());
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Number(Number::from_f64(1.5).unwrap()));
    }

    #[test]
    fn decode_f64_value() {
        let mut buffer = vec![TAG_F64];
        buffer.extend_from_slice(&42.5_f64.to_be_bytes());
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Number(Number::from_f64(42.5).unwrap()));
    }

    #[test]
    fn normalize_float_whole_numbers() {
        let mut buffer = vec![TAG_F32];
        buffer.extend_from_slice(&1.0_f32.to_le_bytes());
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Number(Number::from(1)));

        let mut buffer = vec![TAG_F64];
        buffer.extend_from_slice(&0.0_f64.to_be_bytes());
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Number(Number::from(0)));
    }

    #[test]
    fn keep_float_fractional_numbers() {
        let mut buffer = vec![TAG_F64];
        buffer.extend_from_slice(&10.01_f64.to_be_bytes());
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Number(Number::from_f64(10.01).unwrap()));
    }

    #[test]
    fn decode_string_value() {
        let buffer = encode_string("hello");
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::String("hello".to_string()));
    }

    #[test]
    fn decode_object_simple() {
        let buffer = encode_object(&[("a", vec![TAG_BOOL, 1]), ("b", encode_string("ok"))]);
        let value = decode(&buffer).unwrap();

        let mut expected = Map::new();
        expected.insert("a".to_string(), Value::Bool(true));
        expected.insert("b".to_string(), Value::String("ok".to_string()));
        assert_eq!(value, Value::Object(expected));
    }

    #[test]
    fn decode_nested_object() {
        let inner = encode_object(&[("inner", vec![TAG_BOOL, 1])]);
        let buffer = encode_object(&[("outer", inner)]);
        let value = decode(&buffer).unwrap();

        let mut inner_map = Map::new();
        inner_map.insert("inner".to_string(), Value::Bool(true));
        let mut outer_map = Map::new();
        outer_map.insert("outer".to_string(), Value::Object(inner_map));
        assert_eq!(value, Value::Object(outer_map));
    }

    #[test]
    fn decode_array_values() {
        let buffer = encode_array(&[vec![TAG_BOOL, 1], encode_string("ok")]);
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Array(vec![Value::Bool(true), Value::String("ok".to_string())]));
    }

    #[test]
    fn decode_empty_array() {
        let buffer = vec![TAG_OBJECT, TAG_OBJECT_END];
        let value = decode(&buffer).unwrap();
        assert_eq!(value, Value::Array(Vec::new()));
    }

    #[test]
    fn decode_sequential_numeric_keyed_container_as_array() {
        let buffer = encode_array(&[
            encode_f64(1.0),
            encode_string("first"),
            encode_f64(2.0),
            encode_string("second"),
        ]);

        assert_eq!(decode(&buffer).unwrap(), json!(["first", "second"]));
    }

    #[test]
    fn decode_non_sequential_numeric_keyed_container_as_object() {
        let buffer = encode_array(&[encode_f64(42.0), encode_string("value")]);

        assert_eq!(decode(&buffer).unwrap(), json!({ "42": "value" }));
    }

    #[test]
    fn decode_sequential_numeric_scalar_container_as_array() {
        let buffer =
            encode_array(&[encode_f64(1.0), encode_f64(24.0), encode_f64(2.0), encode_f64(1.0)]);

        assert_eq!(decode(&buffer).unwrap(), json!([24, 1]));
    }

    #[test]
    fn decode_unknown_tag_as_null() {
        let value = decode(&[0x99]).unwrap();
        assert_eq!(value, Value::Null);
    }

    #[test]
    fn decode_skips_preamble_to_valid_payload() {
        let mut buffer = vec![0xff, TAG_BOOL, 0];
        buffer.extend_from_slice(&encode_object(&[("ok", vec![TAG_BOOL, 1])]));
        let value = decode(&buffer).unwrap();

        let mut expected = Map::new();
        expected.insert("ok".to_string(), Value::Bool(true));
        assert_eq!(value, Value::Object(expected));
    }

    #[test]
    fn decode_preamble_without_payload_is_error() {
        let err = decode(&[0x99, 0x88]).unwrap_err();
        assert!(matches!(err, DecodeError::TrailingBytes { .. }));
    }

    #[test]
    fn decode_preamble_scan_is_bounded() {
        let mut buffer = vec![0x99; MAX_PREAMBLE_SCAN_BYTES + 1];
        buffer.extend_from_slice(&encode_object(&[("ok", vec![TAG_BOOL, 1])]));

        let err = decode(&buffer).unwrap_err();
        assert!(matches!(err, DecodeError::TrailingBytes { .. }));
    }

    #[test]
    fn decode_preamble_scan_includes_limit_boundary() {
        let mut buffer = vec![0x99; MAX_PREAMBLE_SCAN_BYTES];
        buffer.extend_from_slice(&encode_object(&[("ok", vec![TAG_BOOL, 1])]));

        let value = decode(&buffer).unwrap();

        let mut expected = Map::new();
        expected.insert("ok".to_string(), Value::Bool(true));
        assert_eq!(value, Value::Object(expected));
    }

    #[test]
    fn decode_invalid_utf8_is_error() {
        let mut buffer = vec![TAG_STRING];
        buffer.extend_from_slice(&2_u32.to_le_bytes());
        buffer.extend_from_slice(&[0xff, 0xff]);
        let err = decode(&buffer).unwrap_err();
        assert!(matches!(err, DecodeError::InvalidUtf8 { .. }));
    }

    #[test]
    fn decode_trailing_bytes_is_error() {
        let mut buffer = encode_object(&[("ok", vec![TAG_BOOL, 1])]);
        buffer.push(TAG_BOOL);
        buffer.push(0);
        let err = decode(&buffer).unwrap_err();
        assert!(matches!(err, DecodeError::TrailingBytes { .. }));
    }

    #[test]
    fn decode_sample_mail_files() {
        let samples = [
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/Battle/Persistent.Mail.485440176891031331"
            ))
            .as_slice(),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/Battle/Persistent.Mail.1409019176893142331"
            ))
            .as_slice(),
        ];

        for sample in samples {
            let value = decode(sample).expect("decode sample");
            assert!(matches!(value, Value::Object(_)));
        }
    }

    #[test]
    fn decode_all_checked_in_mail_samples() {
        let samples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../samples");
        let samples = collect_mail_samples(&samples_dir);

        for input in &samples {
            let buffer = std::fs::read(input)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", input.display()));
            if let Err(error) = decode(&buffer) {
                panic!("failed to decode {}: {error}", input.display());
            }
        }

        assert_eq!(samples.len(), 132);
    }

    fn collect_mail_samples(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut samples = Vec::new();
        collect_mail_samples_inner(dir, &mut samples);
        samples.sort();
        samples
    }

    fn collect_mail_samples_inner(dir: &std::path::Path, samples: &mut Vec<std::path::PathBuf>) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));

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
