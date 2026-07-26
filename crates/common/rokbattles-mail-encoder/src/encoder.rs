//! `Persistent.Mail` value and file encoding.

use serde_json::Value;

use crate::common::{
    CHECKSUM_SEED, EncodeError, FILE_HEADER_LEN, FILE_MARKER, MAX_DEPTH, TABLE_END, TAG_BOOL,
    TAG_F64, TAG_STRING, TAG_TABLE,
};

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
            output.extend_from_slice(&[TAG_BOOL, u8::from(*value)]);
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
    output.push(TAG_TABLE);
    Ok(())
}

fn encode_string(value: &str, output: &mut Vec<u8>) -> Result<(), EncodeError> {
    let length = u32::try_from(value.len()).map_err(|_error| EncodeError::StringTooLong)?;
    output.push(TAG_STRING);
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
    let header_hash = CHECKSUM_SEED
        .wrapping_mul(33)
        .wrapping_add(u64::from(FILE_MARKER))
        .wrapping_mul(33_u64.pow(8));
    rokbattles_djb2_simd::checksum(header_hash, payload)
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
