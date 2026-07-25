//! Encoder for `Persistent.Mail` files.
//!
//! This is the inverse of `rokbattles-mail-decoder`: it writes a JSON value
//! using the game's tagged value format, then adds the fixed file header and
//! checksum.

#![forbid(unsafe_code)]

use serde_json::Value;
use thiserror::Error;

const FILE_MARKER: u8 = 0xff;
const FILE_HEADER_LEN: usize = 9;
const TAG_F64: u8 = 0x03;
const TABLE_END: u8 = 0xff;
const MAX_DEPTH: usize = 128;

/// Encode a JSON value as a complete `Persistent.Mail` file.
///
/// Object fields whose value is `null` are omitted because the persistent
/// format has no null value tag.
/// Empty objects and arrays have the same on-disk representation because the
/// format does not distinguish between empty table kinds.
///
/// # Errors
///
/// Returns an error if the value contains an unrepresentable number, a null
/// outside an object field, an oversized string, or excessive nesting.
pub fn encode(value: &Value) -> Result<Vec<u8>, EncodeError> {
    let mut output = Vec::new();
    output.push(FILE_MARKER);
    output.extend_from_slice(&0_u64.to_le_bytes());
    encode_value(value, &mut output, 0)?;

    let checksum = file_checksum(&output);
    let checksum_field =
        output.get_mut(1..FILE_HEADER_LEN).ok_or(EncodeError::HeaderNotInitialized)?;
    checksum_field.copy_from_slice(&checksum.to_le_bytes());
    Ok(output)
}

fn encode_value(value: &Value, output: &mut Vec<u8>, depth: usize) -> Result<(), EncodeError> {
    match value {
        Value::Null => Err(EncodeError::NullValue),
        Value::Bool(value) => {
            output.extend_from_slice(&[0x01, u8::from(*value)]);
            Ok(())
        }
        Value::Number(value) => {
            let value = value.as_f64().ok_or(EncodeError::UnrepresentableNumber)?;
            if !value.is_finite() {
                return Err(EncodeError::UnrepresentableNumber);
            }
            output.push(TAG_F64);
            output.extend_from_slice(&value.to_be_bytes());
            Ok(())
        }
        Value::String(value) => encode_string(value, output),
        Value::Array(values) => {
            begin_table(output, depth)?;
            for (index, value) in values.iter().enumerate() {
                let key = u64::try_from(index.saturating_add(1))
                    .map_err(|_error| EncodeError::ArrayTooLong)?;
                encode_number_key(key, output);
                encode_value(value, output, depth.saturating_add(1))?;
            }
            output.push(TABLE_END);
            Ok(())
        }
        Value::Object(values) => {
            begin_table(output, depth)?;
            for (key, value) in values {
                if value.is_null() {
                    continue;
                }
                encode_string(key, output)?;
                encode_value(value, output, depth.saturating_add(1))?;
            }
            output.push(TABLE_END);
            Ok(())
        }
    }
}

fn begin_table(output: &mut Vec<u8>, depth: usize) -> Result<(), EncodeError> {
    if depth >= MAX_DEPTH {
        return Err(EncodeError::DepthLimitExceeded { limit: MAX_DEPTH });
    }
    output.push(0x05);
    Ok(())
}

fn encode_string(value: &str, output: &mut Vec<u8>) -> Result<(), EncodeError> {
    let length = u32::try_from(value.len()).map_err(|_error| EncodeError::StringTooLong)?;
    output.push(0x04);
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_number_key(value: u64, output: &mut Vec<u8>) {
    output.push(TAG_F64);
    output.extend_from_slice(&(value as f64).to_be_bytes());
}

fn file_checksum(buffer: &[u8]) -> u64 {
    let payload = buffer.get(FILE_HEADER_LEN..).unwrap_or_default();
    let header_hash = 0x1505_u64
        .wrapping_mul(33)
        .wrapping_add(u64::from(FILE_MARKER))
        .wrapping_mul(33_u64.pow(8));
    rokbattles_djb2_simd::checksum(header_hash, payload)
}

/// Errors returned by [`encode`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodeError {
    /// A null value appeared where it could not be omitted.
    #[error("null cannot be represented by the persistent mail format")]
    NullValue,
    /// A number could not be represented as a finite `f64`.
    #[error("number cannot be represented as a finite f64")]
    UnrepresentableNumber,
    /// A string exceeded the format's `u32` byte-length field.
    #[error("string exceeds the persistent mail limit")]
    StringTooLong,
    /// An array was too large to assign a representable numeric key.
    #[error("array exceeds the persistent mail limit")]
    ArrayTooLong,
    /// A table exceeded the format's defensive nesting limit.
    #[error("table nesting exceeds max depth of {limit}")]
    DepthLimitExceeded {
        /// Maximum supported table nesting depth.
        limit: usize,
    },
    /// The fixed header was not present after initialization.
    #[error("persistent mail file header was not initialized")]
    HeaderNotInitialized,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn encoded_file_roundtrips_through_production_decoder() {
        let value = json!({
            "id": "123",
            "unread": false,
            "body": {
                "content": {
                    "count": 42,
                    "ratio": 1.25,
                    "values": ["a", "b"]
                }
            },
            "attachments": []
        });

        let encoded = encode(&value).expect("value should encode");

        assert_eq!(rokbattles_mail_decoder::decode(&encoded).expect("file should decode"), value);
    }

    #[test]
    fn decoded_real_fixture_can_be_reencoded_without_value_loss() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.10121648172261838131");
        let original = std::fs::read(path).expect("fixture should read");
        let value = rokbattles_mail_decoder::decode(&original).expect("fixture should decode");

        let encoded = encode(&value).expect("fixture should encode");

        assert_eq!(rokbattles_mail_decoder::decode(&encoded).expect("file should decode"), value);
    }

    #[test]
    fn rejects_null_outside_an_object_field() {
        assert_eq!(encode(&Value::Null), Err(EncodeError::NullValue));
    }

    #[test]
    fn rejects_null_inside_an_array() {
        assert_eq!(encode(&json!([null])), Err(EncodeError::NullValue));
    }

    #[test]
    fn omits_null_object_fields() {
        let encoded = encode(&json!({
            "kept": true,
            "omitted": null,
        }))
        .expect("object should encode");

        assert_eq!(
            rokbattles_mail_decoder::decode(&encoded).expect("file should decode"),
            json!({ "kept": true })
        );
    }

    #[test]
    fn empty_objects_and_arrays_share_the_table_representation() {
        assert_eq!(
            encode(&json!({})).expect("object should encode"),
            encode(&json!([])).expect("array should encode")
        );
    }

    #[test]
    fn accepts_values_at_the_nesting_limit() {
        let _encoded =
            encode(&nested_array(MAX_DEPTH)).expect("value at the nesting limit should encode");
    }

    #[test]
    fn rejects_values_beyond_the_nesting_limit() {
        assert_eq!(
            encode(&nested_array(MAX_DEPTH + 1)),
            Err(EncodeError::DepthLimitExceeded { limit: MAX_DEPTH })
        );
    }

    fn nested_array(depth: usize) -> Value {
        let mut value = Value::Bool(true);
        for _ in 0..depth {
            value = Value::Array(vec![value]);
        }
        value
    }
}
