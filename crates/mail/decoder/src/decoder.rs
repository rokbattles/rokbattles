//! Strict `Persistent.Mail` decoder.

use serde_json::{Number, Value};

use crate::{
    common::{
        CHECKSUM_SEED, DecodeError, FILE_HEADER_LEN, FILE_MARKER, MAX_DEPTH, TABLE_END, TAG_BOOL,
        TAG_F64, TAG_STRING, TAG_TABLE,
    },
    value::classify_table,
};

/// Validate the fixed file header and checksum without decoding its value.
///
/// # Errors
///
/// Returns an error when the header is short, its marker is invalid, or its
/// checksum does not match the complete file buffer.
pub fn validate_file(buffer: &[u8]) -> Result<(), DecodeError> {
    if buffer.len() < FILE_HEADER_LEN {
        return Err(DecodeError::HeaderTooShort {
            required: FILE_HEADER_LEN,
            actual: buffer.len(),
        });
    }
    if buffer[0] != FILE_MARKER {
        return Err(DecodeError::InvalidFileMarker { expected: FILE_MARKER, found: buffer[0] });
    }

    let stored = u64::from_le_bytes(bytes8(&buffer[1..FILE_HEADER_LEN])?);
    let computed = file_checksum(buffer);
    if stored != computed {
        return Err(DecodeError::ChecksumMismatch { stored, computed });
    }

    Ok(())
}

/// Decode a complete `Persistent.Mail` file, including header validation.
///
/// # Errors
///
/// Returns an error for an invalid header/checksum, malformed values,
/// unsupported tags, unterminated tables, or trailing bytes.
pub fn decode(buffer: &[u8]) -> Result<Value, DecodeError> {
    validate_file(buffer)?;
    decode_value_at(&buffer[FILE_HEADER_LEN..], FILE_HEADER_LEN)
}

/// Decode exactly one headerless `Persistent.Mail` value.
///
/// This is intended for format tooling and focused tests. Production callers
/// reading `Persistent.Mail` files should use [`decode`] so the file checksum
/// is enforced.
///
/// # Errors
///
/// Returns an error for malformed values, unsupported tags, unterminated
/// tables, or trailing bytes.
pub fn decode_value(buffer: &[u8]) -> Result<Value, DecodeError> {
    decode_value_at(buffer, 0)
}

fn decode_value_at(buffer: &[u8], base_offset: usize) -> Result<Value, DecodeError> {
    let mut decoder = Decoder::new(buffer, base_offset);
    let value = decoder.read_value()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::TrailingBytes { remaining: decoder.remaining() });
    }
    Ok(value)
}

fn file_checksum(buffer: &[u8]) -> u64 {
    buffer.iter().copied().enumerate().fold(CHECKSUM_SEED, |hash, (offset, byte)| {
        let byte = if (1..FILE_HEADER_LEN).contains(&offset) { 0 } else { byte };
        hash.wrapping_mul(33).wrapping_add(u64::from(byte))
    })
}

struct Decoder<'a> {
    buffer: &'a [u8],
    base_offset: usize,
    pos: usize,
    depth: usize,
}

impl<'a> Decoder<'a> {
    fn new(buffer: &'a [u8], base_offset: usize) -> Self {
        Self { buffer, base_offset, pos: 0, depth: 0 }
    }

    fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    fn absolute_offset(&self, relative: usize) -> usize {
        self.base_offset.saturating_add(relative)
    }

    fn read_value(&mut self) -> Result<Value, DecodeError> {
        let tag_offset = self.absolute_offset(self.pos);
        let tag = self.read_u8()?;
        match tag {
            TAG_BOOL => Ok(Value::Bool(self.read_u8()? != 0)),
            TAG_F64 => {
                let raw = self.read_exact(8)?;
                number_value(f64::from_be_bytes(bytes8(raw)?))
            }
            TAG_STRING => Ok(Value::String(self.read_string()?)),
            TAG_TABLE => self.read_table(tag_offset),
            _ => Err(DecodeError::UnsupportedTag { tag, offset: tag_offset }),
        }
    }

    fn read_table(&mut self, table_offset: usize) -> Result<Value, DecodeError> {
        if self.depth >= MAX_DEPTH {
            return Err(DecodeError::DepthLimitExceeded { limit: MAX_DEPTH });
        }

        self.depth += 1;
        let result = self.read_table_contents(table_offset);
        self.depth -= 1;
        result
    }

    fn read_table_contents(&mut self, table_offset: usize) -> Result<Value, DecodeError> {
        let mut items = Vec::new();
        let terminated = loop {
            match self.peek_u8() {
                Some(TABLE_END) => {
                    self.read_u8()?;
                    break true;
                }
                Some(_) => items.push(self.read_value()?),
                None => break false,
            }
        };

        let classified = classify_table(items, table_offset)?;
        if !terminated {
            return Err(DecodeError::MissingTableTerminator { offset: table_offset });
        }

        Ok(classified.value)
    }

    fn read_string(&mut self) -> Result<String, DecodeError> {
        let length = self.read_u32_le()? as usize;
        let remaining = self.remaining();
        if length > remaining {
            return Err(DecodeError::LengthOutOfBounds { length, remaining });
        }

        let start = self.absolute_offset(self.pos);
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
    if !value.is_finite() {
        return Err(DecodeError::NonFiniteNumber { value });
    }
    Ok(Value::Number(normalize_number(value)))
}

fn normalize_number(value: f64) -> Number {
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

    let Some(number) = Number::from_f64(value) else {
        unreachable!("finite f64 is a JSON number");
    };
    number
}

fn to_u64_exact(value: f64) -> Option<u64> {
    if value < 0.0 || value > u64::MAX as f64 {
        return None;
    }
    let int = value as u64;
    ((int as f64) == value).then_some(int)
}

fn to_i64_exact(value: f64) -> Option<i64> {
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return None;
    }
    let int = value as i64;
    ((int as f64) == value).then_some(int)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::*;

    fn encode_string(value: &str) -> Vec<u8> {
        let mut buffer = vec![TAG_STRING];
        buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buffer.extend_from_slice(value.as_bytes());
        buffer
    }

    fn encode_f64(value: f64) -> Vec<u8> {
        let mut buffer = vec![TAG_F64];
        buffer.extend_from_slice(&value.to_be_bytes());
        buffer
    }

    fn encode_table(values: &[Vec<u8>]) -> Vec<u8> {
        let mut buffer = vec![TAG_TABLE];
        for value in values {
            buffer.extend_from_slice(value);
        }
        buffer.push(TABLE_END);
        buffer
    }

    fn encode_object(pairs: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let values = pairs
            .iter()
            .flat_map(|(key, value)| [encode_string(key), value.clone()])
            .collect::<Vec<_>>();
        encode_table(&values)
    }

    fn encode_numeric_table(pairs: &[(f64, Vec<u8>)]) -> Vec<u8> {
        let values = pairs
            .iter()
            .flat_map(|(key, value)| [encode_f64(*key), value.clone()])
            .collect::<Vec<_>>();
        encode_table(&values)
    }

    fn encode_file(value: &[u8]) -> Vec<u8> {
        let mut buffer = vec![FILE_MARKER];
        buffer.extend_from_slice(&0_u64.to_le_bytes());
        buffer.extend_from_slice(value);
        let checksum = file_checksum(&buffer);
        buffer[1..FILE_HEADER_LEN].copy_from_slice(&checksum.to_le_bytes());
        buffer
    }

    #[test]
    fn decode_value_decodes_boolean() {
        assert_eq!(decode_value(&[TAG_BOOL, 1]).expect("decode bool"), Value::Bool(true));
    }

    #[test]
    fn decode_value_decodes_f64() {
        assert_eq!(decode_value(&encode_f64(42.5)).expect("decode f64"), json!(42.5));
    }

    #[test]
    fn decode_value_normalizes_whole_f64() {
        assert_eq!(decode_value(&encode_f64(1.0)).expect("decode f64"), json!(1));
    }

    #[test]
    fn decode_value_decodes_string() {
        assert_eq!(decode_value(&encode_string("hello")).expect("decode string"), json!("hello"));
    }

    #[test]
    fn decode_value_decodes_object_after_complete_table_read() {
        let input = encode_object(&[("a", vec![TAG_BOOL, 1]), ("b", encode_string("ok"))]);

        assert_eq!(decode_value(&input).expect("decode object"), json!({ "a": true, "b": "ok" }));
    }

    #[test]
    fn decode_value_sorts_sequential_numeric_keys() {
        let input =
            encode_numeric_table(&[(2.0, encode_string("second")), (1.0, encode_string("first"))]);

        assert_eq!(decode_value(&input).expect("decode array"), json!(["first", "second"]));
    }

    #[test]
    fn decode_value_preserves_non_sequential_numeric_keys() {
        let input = encode_numeric_table(&[(42.0, encode_string("value"))]);

        assert_eq!(decode_value(&input).expect("decode numeric map"), json!({ "42": "value" }));
    }

    #[test]
    fn decode_value_keeps_unkeyed_sequence() {
        let input = encode_table(&[vec![TAG_BOOL, 1], encode_string("ok")]);

        assert_eq!(decode_value(&input).expect("decode sequence"), json!([true, "ok"]));
    }

    #[test]
    fn decode_value_uses_array_for_empty_table() {
        assert_eq!(decode_value(&[TAG_TABLE, TABLE_END]).expect("decode empty"), json!([]));
    }

    #[test]
    fn decode_accepts_valid_native_header_and_checksum() {
        let file = encode_file(&encode_object(&[("ok", vec![TAG_BOOL, 1])]));

        assert_eq!(decode(&file).expect("decode file"), json!({ "ok": true }));
    }

    #[test]
    fn decode_rejects_headerless_value() {
        let error = decode(&[TAG_BOOL, 1]).expect_err("header should be required");

        assert!(matches!(error, DecodeError::HeaderTooShort { .. }));
    }

    #[test]
    fn decode_rejects_invalid_file_marker() {
        let mut file = encode_file(&encode_string("hello"));
        file[0] = 0;

        assert_eq!(
            decode(&file).expect_err("marker should fail"),
            DecodeError::InvalidFileMarker { expected: FILE_MARKER, found: 0 }
        );
    }

    #[test]
    fn decode_rejects_checksum_corruption() {
        let mut file = encode_file(&encode_string("hello"));
        let last = file.len() - 1;
        file[last] ^= 1;

        assert!(matches!(decode(&file), Err(DecodeError::ChecksumMismatch { .. })));
    }

    #[test]
    fn decode_rejects_unsupported_tag() {
        let file = encode_file(&[0x99]);

        assert_eq!(
            decode(&file).expect_err("unknown tag should fail"),
            DecodeError::UnsupportedTag { tag: 0x99, offset: FILE_HEADER_LEN }
        );
    }

    #[test]
    fn decode_rejects_legacy_f32_tag() {
        let file = encode_file(&[0x02, 0, 0, 0, 0]);

        assert_eq!(
            decode(&file).expect_err("f32 tag should fail"),
            DecodeError::UnsupportedTag { tag: 0x02, offset: FILE_HEADER_LEN }
        );
    }

    #[test]
    fn decode_value_rejects_unknown_tag_inside_table() {
        let input = encode_table(&[vec![0x99]]);

        assert_eq!(
            decode_value(&input).expect_err("unknown tag should fail"),
            DecodeError::UnsupportedTag { tag: 0x99, offset: 1 }
        );
    }

    #[test]
    fn decode_value_rejects_unterminated_keyed_table() {
        let mut input = encode_object(&[("ok", vec![TAG_BOOL, 1])]);
        input.pop();

        assert_eq!(
            decode_value(&input).expect_err("terminator should be required"),
            DecodeError::MissingTableTerminator { offset: 0 }
        );
    }

    #[test]
    fn decode_value_rejects_unterminated_unkeyed_table() {
        let nested = encode_object(&[("type", encode_string("Battle"))]);
        let mut input = vec![TAG_TABLE];
        input.extend_from_slice(&nested);

        assert_eq!(
            decode_value(&input).expect_err("outer table terminator should be required"),
            DecodeError::MissingTableTerminator { offset: 0 }
        );
    }

    #[test]
    fn decode_value_rejects_mixed_table_keys() {
        let input = encode_table(&[
            encode_string("one"),
            vec![TAG_BOOL, 1],
            encode_f64(2.0),
            vec![TAG_BOOL, 0],
        ]);

        assert_eq!(
            decode_value(&input).expect_err("mixed keys should fail"),
            DecodeError::MixedTableKeyTypes { offset: 0 }
        );
    }

    #[test]
    fn decode_value_rejects_duplicate_table_keys() {
        let input = encode_object(&[("same", vec![TAG_BOOL, 1]), ("same", vec![TAG_BOOL, 0])]);

        assert_eq!(
            decode_value(&input).expect_err("duplicate should fail"),
            DecodeError::DuplicateTableKey { offset: 0, key: "same".to_string() }
        );
    }

    #[test]
    fn decode_value_rejects_trailing_bytes() {
        let mut input = encode_object(&[("ok", vec![TAG_BOOL, 1])]);
        input.extend_from_slice(&[TAG_BOOL, 0]);

        assert_eq!(
            decode_value(&input).expect_err("trailing value should fail"),
            DecodeError::TrailingBytes { remaining: 2 }
        );
    }

    #[test]
    fn validate_file_accepts_checked_in_sample() {
        let sample = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/Battle/Persistent.Mail.485440176891031331"
        ));

        assert_eq!(validate_file(sample), Ok(()));
    }

    #[test]
    fn decode_all_checked_in_mail_samples_matches_raw_json() {
        let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../samples");
        let samples = collect_mail_samples(&samples_dir);

        for input in &samples {
            let buffer = std::fs::read(input)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", input.display()));
            let actual = decode(&buffer)
                .unwrap_or_else(|error| panic!("failed to decode {}: {error}", input.display()));
            let expected_path = PathBuf::from(format!("{}.json", input.display()));
            let expected_buffer = std::fs::read(&expected_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", expected_path.display())
            });
            let expected: Value =
                serde_json::from_slice(&expected_buffer).unwrap_or_else(|error| {
                    panic!("failed to parse {}: {error}", expected_path.display())
                });
            assert!(
                json_equivalent(&actual, &expected),
                "decoded output differs for {}: {}",
                input.display(),
                first_json_difference(&actual, &expected, "$"),
            );
        }

        assert_eq!(samples.len(), 132);
    }

    fn collect_mail_samples(dir: &Path) -> Vec<PathBuf> {
        let mut samples = Vec::new();
        collect_mail_samples_inner(dir, &mut samples);
        samples.sort();
        samples
    }

    fn collect_mail_samples_inner(dir: &Path, samples: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));

        for entry in entries {
            let path = entry
                .unwrap_or_else(|error| {
                    panic!("failed to read entry in {}: {error}", dir.display())
                })
                .path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("game") {
                    collect_mail_samples_inner(&path, samples);
                }
            } else if is_binary_mail_sample(&path) {
                samples.push(path);
            }
        }
    }

    fn is_binary_mail_sample(path: &Path) -> bool {
        let Some(file_name) = path.file_name().and_then(std::ffi::OsStr::to_str) else {
            return false;
        };
        file_name.starts_with("Persistent.Mail.") && !file_name.ends_with(".json")
    }

    fn json_equivalent(actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Array(actual), Value::Array(expected)) => {
                actual.len() == expected.len()
                    && actual.iter().zip(expected).all(|(a, b)| json_equivalent(a, b))
            }
            (Value::Object(actual), Value::Object(expected)) => {
                actual.len() == expected.len()
                    && actual.iter().all(|(key, value)| {
                        expected.get(key).is_some_and(|other| json_equivalent(value, other))
                    })
            }
            (Value::Number(actual), Value::Number(expected)) => {
                match (actual.as_i64(), expected.as_i64()) {
                    (Some(actual), Some(expected)) => actual == expected,
                    _ => match (actual.as_u64(), expected.as_u64()) {
                        (Some(actual), Some(expected)) => actual == expected,
                        _ => match (actual.as_f64(), expected.as_f64()) {
                            (Some(actual), Some(expected)) => {
                                ordered_float_bits(actual).abs_diff(ordered_float_bits(expected))
                                    <= 1
                            }
                            _ => false,
                        },
                    },
                }
            }
            _ => actual == expected,
        }
    }

    fn ordered_float_bits(value: f64) -> u64 {
        let bits = value.to_bits();
        if bits & (1_u64 << 63) == 0 { bits | (1_u64 << 63) } else { !bits }
    }

    fn first_json_difference(actual: &Value, expected: &Value, path: &str) -> String {
        match (actual, expected) {
            (Value::Array(actual), Value::Array(expected)) => {
                if actual.len() != expected.len() {
                    return format!("{path} array length {} != {}", actual.len(), expected.len());
                }
                for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    if !json_equivalent(actual, expected) {
                        return first_json_difference(
                            actual,
                            expected,
                            &format!("{path}[{index}]"),
                        );
                    }
                }
            }
            (Value::Object(actual), Value::Object(expected)) => {
                if actual.len() != expected.len() {
                    return format!("{path} object length {} != {}", actual.len(), expected.len());
                }
                for (key, actual) in actual {
                    let Some(expected) = expected.get(key) else {
                        return format!("{path}.{key} is missing from expected output");
                    };
                    if !json_equivalent(actual, expected) {
                        return first_json_difference(actual, expected, &format!("{path}.{key}"));
                    }
                }
            }
            _ => return format!("{path}: actual {actual:?}, expected {expected:?}"),
        }
        "no structural difference found".to_string()
    }
}
