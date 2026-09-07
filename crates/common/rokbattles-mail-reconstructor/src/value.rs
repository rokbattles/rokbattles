//! Shared decompression and persistent JSON value conventions.
//!
//! Lua tables use arrays for empty tables and may arrive as alternating one-based
//! indices and values. Normalization applies these conventions recursively and
//! removes null object members; it is not a lossless JSON transformation.

use std::io::Read;

use flate2::read::ZlibDecoder;
use serde_json::{Value, json};

use crate::ReconstructionError;

/// Inflates zlib bytes and requires the declared length to match exactly.
///
/// Missing, negative, or above-limit declarations fail before decompression.
pub(crate) fn inflate_mail_body(
    compressed: &[u8],
    original_length: Option<i32>,
    max: usize,
) -> Result<Vec<u8>, ReconstructionError> {
    let expected = original_length
        .and_then(|length| usize::try_from(length).ok())
        .filter(|length| *length <= max)
        .ok_or(ReconstructionError::InvalidInflatedLength)?;
    // Reading one byte past the cap makes oversized output a length mismatch
    // without retaining the rest of the decompressed stream.
    let mut decoder = ZlibDecoder::new(compressed).take((max + 1) as u64);
    let mut inflated = Vec::with_capacity(expected);
    decoder.read_to_end(&mut inflated).map_err(ReconstructionError::Inflate)?;
    if inflated.len() != expected {
        return Err(ReconstructionError::InflatedLengthMismatch {
            expected,
            actual: inflated.len(),
        });
    }
    Ok(inflated)
}

/// Keeps JSON objects; other inputs become a name with an empty alliance tag.
///
/// Even valid JSON scalars and arrays use the original text as the name.
pub(crate) fn decode_info(value: &str) -> Value {
    match serde_json::from_str::<Value>(value) {
        Ok(Value::Object(object)) => Value::Object(object),
        _ => json!({ "Name": value, "Abbr": "" }),
    }
}

/// Converts comma-separated names to true-valued keys without trimming them.
///
/// Empty input becomes an array; nonempty input becomes an object, ignoring
/// empty segments and collapsing duplicate names.
pub(crate) fn decode_flags(value: &str) -> Value {
    if value.is_empty() {
        return Value::Array(Vec::new());
    }
    let flags = value
        .split(',')
        .filter(|flag| !flag.is_empty())
        .map(|flag| (flag.to_string(), Value::Bool(true)))
        .collect();
    Value::Object(flags)
}

/// Removes null object members, maps empty objects to arrays, and collapses
/// arrays shaped like `[1, value, 2, value, ...]` into their values.
///
/// Empty arrays and null array elements are retained.
pub(crate) fn normalize_lua_table(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values.iter_mut() {
                normalize_lua_table(value);
            }
            // Require the full one-based sequence before discarding index slots.
            let is_indexed_table = !values.is_empty()
                && values.len().is_multiple_of(2)
                && values
                    .iter()
                    .step_by(2)
                    .enumerate()
                    .all(|(index, value)| value.as_u64() == u64::try_from(index + 1).ok());
            if is_indexed_table {
                *values = values
                    .drain(..)
                    .enumerate()
                    .filter_map(|(index, value)| (index % 2 == 1).then_some(value))
                    .collect();
            }
        }
        Value::Object(values) => {
            values.retain(|_key, value| !value.is_null());
            for value in values.values_mut() {
                normalize_lua_table(value);
            }
            if values.is_empty() {
                *value = Value::Array(Vec::new());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::ZlibEncoder};
    use serde_json::json;

    use super::*;

    fn compress(value: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(value).expect("value should compress");
        encoder.finish().expect("compression should finish")
    }

    #[test]
    fn inflates_body_with_matching_declared_length() {
        let body = b"mail body";
        let inflated = inflate_mail_body(
            &compress(body),
            Some(i32::try_from(body.len()).expect("test body should fit")),
            64,
        )
        .expect("body should inflate");

        assert_eq!(inflated, body);
    }

    #[test]
    fn rejects_invalid_or_oversized_inflated_lengths() {
        let compressed = compress(b"mail");

        assert!(matches!(
            inflate_mail_body(&compressed, None, 64),
            Err(ReconstructionError::InvalidInflatedLength)
        ));
        assert!(matches!(
            inflate_mail_body(&compressed, Some(-1), 64),
            Err(ReconstructionError::InvalidInflatedLength)
        ));
        assert!(matches!(
            inflate_mail_body(&compressed, Some(65), 64),
            Err(ReconstructionError::InvalidInflatedLength)
        ));
    }

    #[test]
    fn rejects_inflated_length_mismatch() {
        assert!(matches!(
            inflate_mail_body(&compress(b"mail"), Some(3), 64),
            Err(ReconstructionError::InflatedLengthMismatch { expected: 3, actual: 4 })
        ));
    }

    #[test]
    fn decodes_structured_info_and_falls_back_for_plain_text() {
        assert_eq!(
            decode_info(r#"{"Name":"system","Abbr":"SYS"}"#),
            json!({"Name": "system", "Abbr": "SYS"})
        );
        assert_eq!(decode_info("system"), json!({"Name": "system", "Abbr": ""}));
        assert_eq!(decode_info(""), json!({"Name": "", "Abbr": ""}));
    }

    #[test]
    fn decodes_empty_and_comma_separated_flags() {
        assert_eq!(decode_flags(""), json!([]));
        assert_eq!(decode_flags("read,,starred"), json!({"read": true, "starred": true}));
    }

    #[test]
    fn normalizes_nested_lua_table_shapes() {
        let mut value = json!({
            "indexed": [1, {"keep": 1, "drop": null}, 2, []],
            "empty": {},
            "null": null
        });

        normalize_lua_table(&mut value);

        assert_eq!(
            value,
            json!({
                "indexed": [{"keep": 1}, []],
                "empty": []
            })
        );
    }

    #[test]
    fn leaves_nonsequential_index_pairs_unchanged() {
        let mut value = json!([1, "first", 3, "third"]);

        normalize_lua_table(&mut value);

        assert_eq!(value, json!([1, "first", 3, "third"]));
    }
}
