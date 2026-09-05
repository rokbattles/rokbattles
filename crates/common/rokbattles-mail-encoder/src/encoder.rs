//! Builds a file header and recursively writes tagged values.
//!
//! `encode` owns the output buffer and fills in its checksum after all values
//! have been written. The recursive helpers append to that buffer, passing the
//! number of enclosing tables as `depth`. A failed write may leave partial
//! bytes in the buffer, which `encode` discards when returning the error.

use serde_json::Value;

use crate::common::{
    CHECKSUM_SEED, EncodeError, FILE_HEADER_LEN, FILE_MARKER, MAX_DEPTH, TABLE_END, TAG_BOOL,
    TAG_F64, TAG_STRING, TAG_TABLE,
};

/// Encodes a JSON value as a complete `Persistent.Mail` file.
///
/// Returns an owned buffer containing the header, checksum, and one tagged
/// value. The input is left unchanged, and no file is written to disk.
///
/// Null object fields are omitted, empty objects and arrays have the same
/// representation, and numbers are converted to `f64`, which can round large
/// integers. See the [conversion rules](crate#tables-and-conversion) for details.
///
/// # Errors
///
/// Returns an error for a null root or array element, a number that cannot be
/// converted to a finite `f64`, or a string or object key longer than
/// `u32::MAX` bytes. Also rejects array indices that cannot be converted to
/// `u64` and objects or arrays nested more than 128 levels deep. No partial
/// output is returned on error. See [`EncodeError`] for the individual cases.
///
/// # Examples
///
/// ```no_run
/// use rokbattles_mail_encoder::encode;
/// use serde_json::json;
///
/// let value = json!({ "id": "123", "unread": false });
/// let bytes = encode(&value)?;
/// std::fs::write("Persistent.Mail.123", bytes)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn encode(value: &Value) -> Result<Vec<u8>, EncodeError> {
    let mut output = Vec::new();
    // Reserve the checksum field so the payload begins at its final file offset.
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
            // The format has one numeric tag. Conversion may round JSON integers;
            // only failed conversions and non-finite results are rejected.
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
            // Explicit one-based keys let the decoder recover array positions
            // without mistaking the elements themselves for key/value pairs.
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
                // Omit the key too: writing it before skipping null would leave
                // an unmatched key in the table.
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
    // `depth` counts enclosing tables, so opening a table here adds one level.
    if depth >= MAX_DEPTH {
        return Err(EncodeError::DepthLimitExceeded { limit: MAX_DEPTH });
    }
    output.push(TAG_TABLE);
    Ok(())
}

fn encode_string(value: &str, output: &mut Vec<u8>) -> Result<(), EncodeError> {
    // The length prefix counts UTF-8 bytes, not Unicode characters.
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
    // `encode` supplies the fixed marker and header. Hash the marker followed
    // by eight zero bytes; each zero advances DJB2 by a factor of 33. This also
    // makes the result independent of any checksum already stored in `buffer`.
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
