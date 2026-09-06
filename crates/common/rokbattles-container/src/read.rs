use crate::{Error, HEADER_LEN, Header, MAGIC, VERSION, mask, schema};

/// Bounds enforced before copying, unmasking, or decoding a payload.
#[derive(Debug, Clone, Copy)]
pub struct ReadLimits {
    /// Maximum payload bytes, excluding the header; defaults to 16 MiB.
    /// This is an input bound, not a bound on the size of decoded objects.
    pub max_payload_len: u32,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self { max_payload_len: 16 * 1024 * 1024 }
    }
}

/// A validated envelope with its mask removed, before schema decoding.
#[derive(Debug)]
pub struct Envelope {
    /// Validated header metadata.
    pub header: Header,
    /// Unmasked, checksum-verified bytes.
    pub payload: Vec<u8>,
}

/// A value decoded using one of the built-in schemas.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// Raw bytes (schema 1).
    Bytes(Vec<u8>),
    /// UTF-8 text (schema 2).
    Text(String),
    /// Parsed JSON (schema 3).
    Json(serde_json::Value),
}

/// Header metadata and a schema-decoded value.
#[derive(Debug)]
pub struct Decoded<T = Value> {
    /// Header associated with the decoded value.
    pub header: Header,
    /// Result of the schema decoder.
    pub value: T,
}

/// Reads complete ROKB files using configurable payload limits.
#[derive(Debug, Default, Clone, Copy)]
pub struct Reader {
    limits: ReadLimits,
}

impl Reader {
    /// Creates a reader with explicit limits.
    pub const fn new(limits: ReadLimits) -> Self {
        Self { limits }
    }

    /// Validates and unmasks one complete file, including its payload checksum.
    ///
    /// Unknown nonzero schema IDs are allowed here for application codecs.
    /// The returned payload owns a copy of the input bytes with both masking
    /// stages reversed. The checksum covers the payload, not the header.
    ///
    /// # Errors
    ///
    /// Rejects invalid headers, unsupported versions or flags, oversized input,
    /// length mismatches, and checksum failures.
    pub fn read_envelope(&self, bytes: &[u8]) -> Result<Envelope, Error> {
        let (raw, encoded) =
            bytes.split_first_chunk::<HEADER_LEN>().ok_or(Error::TruncatedHeader)?;
        let &[
            m0,
            m1,
            m2,
            m3,
            version,
            flags,
            s0,
            s1,
            n0,
            n1,
            n2,
            n3,
            l0,
            l1,
            l2,
            l3,
            c0,
            c1,
            c2,
            c3,
        ] = raw;
        if [m0, m1, m2, m3] != MAGIC {
            return Err(Error::InvalidMagic);
        }
        if version != VERSION {
            return Err(Error::UnsupportedVersion(version));
        }
        if flags != 0 {
            return Err(Error::UnsupportedFlags(flags));
        }
        let header = Header {
            version,
            flags,
            schema_id: u16::from_le_bytes([s0, s1]),
            seed: u32::from_le_bytes([n0, n1, n2, n3]),
            payload_len: u32::from_le_bytes([l0, l1, l2, l3]),
            crc32: u32::from_le_bytes([c0, c1, c2, c3]),
        };
        if header.schema_id == 0 {
            return Err(Error::InvalidSchema);
        }
        if header.payload_len > self.limits.max_payload_len {
            return Err(Error::PayloadTooLarge);
        }
        if u32::try_from(encoded.len()).ok() != Some(header.payload_len) {
            return Err(Error::LengthMismatch);
        }
        let mut payload = encoded.to_vec();
        mask::decode(&mut payload, header.seed);
        if crc32fast::hash(&payload) != header.crc32 {
            return Err(Error::ChecksumMismatch);
        }
        Ok(Envelope { header, payload })
    }

    /// Decodes a file using the built-in schema identified by its header.
    ///
    /// # Errors
    ///
    /// Returns envelope validation errors, [`Error::UnknownSchema`] for an
    /// unsupported schema, or an error for invalid UTF-8 or JSON.
    pub fn decode(&self, bytes: &[u8]) -> Result<Decoded, Error> {
        self.decode_expected(bytes, None)
    }

    /// Decodes a file, optionally requiring a particular built-in schema.
    ///
    /// Passing `None` has the same behavior as [`Self::decode`].
    ///
    /// # Errors
    ///
    /// Returns [`Self::decode`]'s errors or [`Error::SchemaMismatch`] if the
    /// validated envelope has a different ID. The schema check precedes payload
    /// interpretation.
    pub fn decode_expected(&self, bytes: &[u8], expected: Option<u16>) -> Result<Decoded, Error> {
        let envelope = self.read_envelope(bytes)?;
        if let Some(expected) = expected
            && expected != envelope.header.schema_id
        {
            return Err(Error::SchemaMismatch { expected, actual: envelope.header.schema_id });
        }
        let value = match envelope.header.schema_id {
            schema::BYTES => Value::Bytes(envelope.payload),
            schema::TEXT => Value::Text(std::str::from_utf8(&envelope.payload)?.to_owned()),
            schema::JSON => Value::Json(serde_json::from_slice(&envelope.payload)?),
            id => return Err(Error::UnknownSchema(id)),
        };
        Ok(Decoded { header: envelope.header, value })
    }

    /// Decodes a file with a caller-supplied schema decoder.
    ///
    /// The callback receives the schema ID from the header and must reject IDs
    /// it does not support. It must validate the payload's fields, enforce any
    /// allocation limits, and consume the complete payload.
    ///
    /// # Errors
    ///
    /// Returns envelope validation errors or the callback's error. Validation
    /// failures prevent the callback from running.
    pub fn decode_with<T>(
        &self,
        bytes: &[u8],
        decoder: impl FnOnce(u16, &[u8]) -> Result<T, Error>,
    ) -> Result<Decoded<T>, Error> {
        let envelope = self.read_envelope(bytes)?;
        let value = decoder(envelope.header.schema_id, &envelope.payload)?;
        Ok(Decoded { header: envelope.header, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data::VECTORS;
    #[cfg(feature = "write")]
    use crate::{write_bytes, write_envelope, write_text};

    #[test]
    fn decodes_known_wire_values() {
        for &(schema_id, payload, file) in VECTORS {
            let decoded = Reader::default().decode(file).expect("decode");
            let expected = match schema_id {
                schema::BYTES => Value::Bytes(payload.to_vec()),
                schema::TEXT => {
                    Value::Text(std::str::from_utf8(payload).expect("UTF-8").to_owned())
                }
                schema::JSON => Value::Json(serde_json::from_slice(payload).expect("JSON")),
                other => panic!("unexpected schema {other}"),
            };
            assert_eq!(decoded.header.schema_id, schema_id);
            assert_eq!(decoded.value, expected);
        }
    }

    #[test]
    fn rejects_every_truncation() {
        for &(_, _, file) in VECTORS {
            for length in 0..file.len() {
                Reader::default()
                    .decode(file.get(..length).expect("prefix"))
                    .expect_err("truncated");
            }
        }
    }

    #[test]
    fn rejects_each_single_byte_payload_corruption() {
        for &(_, _, file) in VECTORS {
            for index in HEADER_LEN..file.len() {
                let mut corrupted = file.to_vec();
                *corrupted.get_mut(index).expect("payload byte") ^= 1;
                assert!(matches!(
                    Reader::default().decode(&corrupted),
                    Err(Error::ChecksumMismatch)
                ));
            }
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn custom_rust_decoder_dispatches_using_the_header() {
        let file = write_envelope(256, &(-1234_i32).to_le_bytes(), 1).expect("write");
        let decoded = Reader::default()
            .decode_with(&file, |id, payload| {
                if id != 256 {
                    return Err(Error::UnknownSchema(id));
                }
                let bytes = payload
                    .try_into()
                    .map_err(|_length| Error::InvalidPayload("expected i32".into()))?;
                Ok(i32::from_le_bytes(bytes))
            })
            .expect("custom decode");
        assert_eq!(decoded.value, -1234);
    }

    #[cfg(feature = "write")]
    #[test]
    fn unknown_schema_is_not_guessed() {
        let file = write_envelope(256, b"hello", 0).expect("write");
        assert!(matches!(Reader::default().decode(&file), Err(Error::UnknownSchema(256))));
    }

    #[cfg(feature = "write")]
    #[test]
    fn expected_schema_prevents_interpreting_a_different_payload() {
        let file = write_text("hello", 1).expect("write");
        assert!(matches!(
            Reader::default().decode_expected(&file, Some(schema::JSON)),
            Err(Error::SchemaMismatch { expected: 3, actual: 2 })
        ));
    }

    #[cfg(feature = "write")]
    #[test]
    fn limits_reject_payload_before_decoding() {
        let file = write_bytes(&[0; 10], 1).expect("write");
        let reader = Reader::new(ReadLimits { max_payload_len: 9 });
        assert!(matches!(reader.decode(&file), Err(Error::PayloadTooLarge)));
        Reader::new(ReadLimits { max_payload_len: 10 }).decode(&file).expect("exact limit");
    }

    #[cfg(feature = "write")]
    #[test]
    fn invalid_utf8_is_rejected_even_with_a_valid_checksum() {
        let file = write_envelope(schema::TEXT, &[0xff], 1).expect("write");
        assert!(matches!(Reader::default().decode(&file), Err(Error::InvalidUtf8(_))));
    }

    #[cfg(feature = "write")]
    #[test]
    fn invalid_or_excessively_nested_json_is_rejected() {
        let deeply_nested = [b"[".repeat(200), b"0".to_vec(), b"]".repeat(200)].concat();
        for json in [b"{}{}".to_vec(), deeply_nested, b"NaN".to_vec(), vec![0xff]] {
            let file = write_envelope(schema::JSON, &json, 0).expect("write");
            assert!(matches!(Reader::default().decode(&file), Err(Error::Json(_))));
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn payload_is_always_masked_and_flags_are_reserved() {
        let file = write_bytes(b"hello", 0).expect("write");
        assert_eq!(file.get(5), Some(&0));
        assert_ne!(file.get(20..).expect("payload"), b"hello");
        assert_eq!(Reader::default().read_envelope(&file).expect("read").payload, b"hello");
    }

    #[cfg(feature = "write")]
    #[test]
    fn every_reserved_flag_is_rejected() {
        let mut file = write_bytes(b"hello", 42).expect("write");
        for flags in 1..=u8::MAX {
            *file.get_mut(5).expect("flags byte") = flags;
            assert!(
                matches!(Reader::default().decode(&file), Err(Error::UnsupportedFlags(actual)) if actual == flags)
            );
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn plaintext_cannot_bypass_the_mandatory_mask() {
        let mut file = write_bytes(b"hello", 0).expect("write");
        file.get_mut(20..).expect("payload").copy_from_slice(b"hello");
        assert!(matches!(Reader::default().decode(&file), Err(Error::ChecksumMismatch)));
    }

    #[cfg(feature = "write")]
    #[test]
    fn header_and_framing_errors_are_distinct() {
        let original = write_bytes(b"hello", 0).expect("write");
        for (offset, byte, expected) in [
            (0, b'X', "invalid ROKB magic"),
            (4, 2, "unsupported ROKB version 2"),
            (5, 128, "unsupported ROKB flags 0x80"),
            (5, 2, "unsupported ROKB flags 0x02"),
            (6, 0, "ROKB schema ID zero is reserved"),
            (8, 1, "ROKB payload checksum failed"),
            (12, 4, "ROKB payload length mismatch"),
            (16, 0, "ROKB payload checksum failed"),
        ] {
            let mut file = original.clone();
            *file.get_mut(offset).expect("header byte") = byte;
            assert_eq!(
                Reader::default().decode(&file).expect_err("invalid file").to_string(),
                expected
            );
        }
        let mut trailing = original;
        trailing.push(0);
        assert!(matches!(Reader::default().decode(&trailing), Err(Error::LengthMismatch)));
    }
}
