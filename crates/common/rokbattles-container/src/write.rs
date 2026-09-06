use crate::{Error, HEADER_LEN, MAGIC, VERSION, mask, schema};

/// Writes serialized payload bytes with a nonzero schema ID.
///
/// Always applies XOR then layered masking with the supplied seed, including zero.
/// Flags are reserved and written as zero.
///
/// The caller must serialize the payload according to `schema_id`; this
/// function writes the envelope without interpreting those bytes.
///
/// # Errors
///
/// Returns [`Error::InvalidSchema`] for ID zero and [`Error::PayloadTooLarge`]
/// if the length exceeds the wire format or the output allocation fails.
pub fn write_envelope(schema_id: u16, payload: &[u8], seed: u32) -> Result<Vec<u8>, Error> {
    if schema_id == 0 {
        return Err(Error::InvalidSchema);
    }
    let length = u32::try_from(payload.len()).map_err(|_overflow| Error::PayloadTooLarge)?;
    let capacity = payload.len().checked_add(HEADER_LEN).ok_or(Error::PayloadTooLarge)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_allocation| Error::PayloadTooLarge)?;
    bytes.extend_from_slice(&MAGIC);
    bytes.push(VERSION);
    bytes.push(0); // Flags are reserved for future functionality.
    bytes.extend_from_slice(&schema_id.to_le_bytes());
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
    bytes.extend_from_slice(payload);
    let encoded = bytes.get_mut(HEADER_LEN..).ok_or(Error::LengthMismatch)?;
    mask::encode(encoded, seed);
    Ok(bytes)
}

/// Writes a byte payload using [`schema::BYTES`].
///
/// Decoding recovers the exact input bytes. Masking and errors follow
/// [`write_envelope`].
pub fn write_bytes(value: &[u8], seed: u32) -> Result<Vec<u8>, Error> {
    write_envelope(schema::BYTES, value, seed)
}

/// Writes UTF-8 text using [`schema::TEXT`], without normalization.
///
/// Masking and errors follow [`write_envelope`].
pub fn write_text(value: &str, seed: u32) -> Result<Vec<u8>, Error> {
    write_envelope(schema::TEXT, value.as_bytes(), seed)
}

/// Serializes a JSON value using [`schema::JSON`].
///
/// Serialization uses serde_json's representation of the value. Use
/// [`write_text`] or [`write_bytes`] to preserve original JSON source text.
///
/// # Errors
///
/// Returns serialization errors or errors from [`write_envelope`].
pub fn write_json(value: &serde_json::Value, seed: u32) -> Result<Vec<u8>, Error> {
    write_envelope(schema::JSON, &serde_json::to_vec(value)?, seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data::VECTORS;
    #[cfg(feature = "read")]
    use crate::{Reader, Value};

    #[test]
    fn matches_known_wire_bytes() {
        for &(schema_id, payload, expected) in VECTORS {
            assert_eq!(write_envelope(schema_id, payload, 0).expect("write"), expected);
        }
    }
    #[test]
    fn schema_zero_cannot_be_written() {
        assert!(matches!(write_envelope(0, &[], 0), Err(Error::InvalidSchema)));
    }

    #[cfg(feature = "read")]
    #[test]
    fn typed_writers_dispatch_without_a_caller_schema() {
        let json = serde_json::json!({"id": u64::MAX, "values": [1, -2, 3.5]});
        let cases = [
            (write_text("\0ROK Battles🏰", 0), Value::Text("\0ROK Battles🏰".into())),
            (write_json(&json, 0), Value::Json(json)),
        ];
        for (encoded, expected) in cases {
            assert_eq!(
                Reader::default().decode(&mut encoded.expect("write")).expect("read").value,
                expected
            );
        }
    }
}
