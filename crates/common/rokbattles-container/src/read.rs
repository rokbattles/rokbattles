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
pub struct Envelope<'a> {
    /// Validated header metadata.
    pub header: Header,
    /// Unmasked, checksum-verified bytes.
    pub payload: &'a [u8],
}

/// A value decoded using one of the enabled schemas.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// Raw bytes (schema 1).
    Bytes(Vec<u8>),
    /// UTF-8 text (schema 2).
    Text(String),
    /// Parsed JSON (schema 3).
    Json(serde_json::Value),
    /// Territory mesh definitions (schema 401).
    #[cfg(feature = "schemas")]
    TerritoryMesh(Vec<crate::schemas::territory::MeshDefinition>),
    /// Territory spatial chunk (schema 402).
    #[cfg(feature = "schemas")]
    TerritoryChunk(crate::schemas::territory::SpatialChunk),
    /// Territory province grid (schema 403).
    #[cfg(feature = "schemas")]
    TerritoryProvince(crate::schemas::territory::ProvinceGrid),
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

    /// Validates a file and unmasks its payload in the supplied buffer.
    ///
    /// Returns a checksum-verified payload borrowed from `bytes`, without
    /// allocating or copying it. Nonzero application schema IDs are accepted.
    /// The header bytes are left unchanged; the checksum covers only the payload.
    ///
    /// # Errors
    ///
    /// Rejects invalid headers, unsupported versions or flags, oversized input,
    /// length mismatches, and checksum failures. Header and framing errors leave
    /// the buffer unchanged. A checksum failure leaves the payload
    /// modified; discard it or reload the original file before retrying.
    pub fn read_envelope<'a>(&self, bytes: &'a mut [u8]) -> Result<Envelope<'a>, Error> {
        let header = self.read_header(bytes)?;
        let payload = bytes.get_mut(HEADER_LEN..).ok_or(Error::TruncatedHeader)?;
        decode_payload(payload, header)?;
        Ok(Envelope { header, payload })
    }

    /// Unmasks a file in place and decodes its enabled payload schema.
    ///
    /// The decoded value owns its data. Use [`Self::read_envelope`] to borrow
    /// the payload instead. The input remains unmasked after a successful read.
    ///
    /// # Errors
    ///
    /// Returns envelope validation errors, [`Error::UnknownSchema`] for an
    /// unsupported schema, or an error for invalid payload fields or exceeded
    /// schema limits. As with
    /// [`Self::read_envelope`], errors after unmasking leave the input modified;
    /// reload the original file before retrying.
    pub fn decode(&self, bytes: &mut [u8]) -> Result<Decoded, Error> {
        self.decode_expected(bytes, None)
    }

    /// Decodes a file, optionally requiring a particular enabled schema.
    ///
    /// Passing `None` has the same behavior as [`Self::decode`].
    ///
    /// # Errors
    ///
    /// Returns [`Self::decode`]'s errors or [`Error::SchemaMismatch`] if the
    /// validated envelope has a different ID. The schema check precedes payload
    /// interpretation, after unmasking. A schema mismatch leaves the input
    /// unmasked; reload the original file before retrying.
    pub fn decode_expected(
        &self,
        bytes: &mut [u8],
        expected: Option<u16>,
    ) -> Result<Decoded, Error> {
        let envelope = self.read_envelope(bytes)?;
        if let Some(expected) = expected
            && expected != envelope.header.schema_id
        {
            return Err(Error::SchemaMismatch { expected, actual: envelope.header.schema_id });
        }
        let value = match envelope.header.schema_id {
            schema::BYTES => Value::Bytes(envelope.payload.to_vec()),
            schema::TEXT => Value::Text(std::str::from_utf8(envelope.payload)?.to_owned()),
            schema::JSON => Value::Json(serde_json::from_slice(envelope.payload)?),
            #[cfg(feature = "schemas")]
            id => crate::schemas::territory::decode(id, envelope.payload)?,
            #[cfg(not(feature = "schemas"))]
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
    /// failures prevent the callback from running. Input mutation follows
    /// [`Self::read_envelope`]; a callback error leaves the input unmasked.
    pub fn decode_with<T>(
        &self,
        bytes: &mut [u8],
        decoder: impl FnOnce(u16, &[u8]) -> Result<T, Error>,
    ) -> Result<Decoded<T>, Error> {
        let envelope = self.read_envelope(bytes)?;
        let value = decoder(envelope.header.schema_id, envelope.payload)?;
        Ok(Decoded { header: envelope.header, value })
    }

    fn read_header(&self, bytes: &[u8]) -> Result<Header, Error> {
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
        Ok(header)
    }
}

fn decode_payload(payload: &mut [u8], header: Header) -> Result<(), Error> {
    mask::decode(payload, header.seed);
    if crc32fast::hash(payload) != header.crc32 {
        return Err(Error::ChecksumMismatch);
    }
    Ok(())
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
            let decoded = Reader::default().decode(&mut file.to_vec()).expect("decode");
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
    fn in_place_payload_borrows_the_original_buffer() {
        for &(schema_id, payload, file) in VECTORS {
            let mut bytes = file.to_vec();
            let payload_ptr = bytes.get(HEADER_LEN..).expect("payload").as_ptr();
            let decoded = Reader::default().read_envelope(&mut bytes).expect("decode");
            assert_eq!(decoded.header.schema_id, schema_id);
            assert_eq!(decoded.payload, payload);
            assert_eq!(decoded.payload.as_ptr(), payload_ptr);
            assert_eq!(bytes.get(..HEADER_LEN), file.get(..HEADER_LEN));
        }
    }

    #[test]
    fn in_place_header_errors_leave_input_unchanged() {
        let file = VECTORS.first().expect("vector").2;
        for (offset, value) in [(0, b'X'), (4, 2), (5, 1), (6, 0), (12, 1)] {
            let mut bytes = file.to_vec();
            *bytes.get_mut(offset).expect("header byte") = value;
            let original = bytes.clone();
            let expected =
                Reader::default().decode(&mut bytes.clone()).expect_err("invalid header");
            let error = Reader::default().read_envelope(&mut bytes).expect_err("invalid header");
            assert_eq!(error.to_string(), expected.to_string());
            assert_eq!(bytes, original);
        }
    }

    #[test]
    fn rejects_every_truncation() {
        for &(_, _, file) in VECTORS {
            for length in 0..file.len() {
                Reader::default()
                    .decode(&mut file.get(..length).expect("prefix").to_vec())
                    .expect_err("truncated");
                let prefix = file.get(..length).expect("prefix");
                let mut bytes = prefix.to_vec();
                Reader::default().read_envelope(&mut bytes).expect_err("truncated");
                assert_eq!(bytes, prefix);
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
                    Reader::default().decode(&mut corrupted.clone()),
                    Err(Error::ChecksumMismatch)
                ));
                assert!(matches!(
                    Reader::default().read_envelope(&mut corrupted),
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
            .decode_with(&mut file.clone(), |id, payload| {
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
        assert!(matches!(
            Reader::default().decode(&mut file.clone()),
            Err(Error::UnknownSchema(256))
        ));
    }

    #[cfg(all(feature = "write", not(feature = "schemas")))]
    #[test]
    fn application_schemas_remain_raw_without_the_feature() {
        let file = write_envelope(401, &[1, 0], 42).expect("write");
        assert!(matches!(
            Reader::default().decode(&mut file.clone()),
            Err(Error::UnknownSchema(401))
        ));
        let mut raw = file;
        let envelope = Reader::default().read_envelope(&mut raw).expect("envelope");
        assert_eq!(envelope.payload, &[1, 0]);
    }

    #[cfg(feature = "write")]
    #[test]
    fn expected_schema_prevents_interpreting_a_different_payload() {
        let mut file = write_text("hello", 1).expect("write");
        assert!(matches!(
            Reader::default().decode_expected(&mut file, Some(schema::JSON)),
            Err(Error::SchemaMismatch { expected: 3, actual: 2 })
        ));
        assert_eq!(file.get(HEADER_LEN..).expect("payload"), b"hello");
    }

    #[cfg(feature = "write")]
    #[test]
    fn callback_errors_leave_payload_unmasked() {
        let mut file = write_envelope(401, b"hello", 1).expect("write");
        let result = Reader::default().decode_with(&mut file, |id, payload| {
            assert_eq!(id, 401);
            assert_eq!(payload, b"hello");
            Err::<(), _>(Error::InvalidPayload("rejected".into()))
        });
        assert!(matches!(result, Err(Error::InvalidPayload(_))));
        assert_eq!(file.get(HEADER_LEN..).expect("payload"), b"hello");
    }

    #[cfg(feature = "write")]
    #[test]
    fn limits_reject_payload_before_decoding() {
        let file = write_bytes(&[0; 10], 1).expect("write");
        let reader = Reader::new(ReadLimits { max_payload_len: 9 });
        assert!(matches!(reader.decode(&mut file.clone()), Err(Error::PayloadTooLarge)));
        let mut bytes = file.clone();
        assert!(matches!(reader.read_envelope(&mut bytes), Err(Error::PayloadTooLarge)));
        assert_eq!(bytes, file);
        Reader::new(ReadLimits { max_payload_len: 10 })
            .decode(&mut file.clone())
            .expect("exact limit");
    }

    #[cfg(feature = "write")]
    #[test]
    fn in_place_accepts_empty_custom_payloads() {
        for seed in [0, 42, u32::MAX] {
            let mut file = write_envelope(401, &[], seed).expect("write");
            let decoded = Reader::new(ReadLimits { max_payload_len: 0 })
                .read_envelope(&mut file)
                .expect("empty payload");
            assert_eq!(decoded.header.schema_id, 401);
            assert!(decoded.payload.is_empty());
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn invalid_utf8_is_rejected_even_with_a_valid_checksum() {
        let file = write_envelope(schema::TEXT, &[0xff], 1).expect("write");
        assert!(matches!(Reader::default().decode(&mut file.clone()), Err(Error::InvalidUtf8(_))));
    }

    #[cfg(feature = "write")]
    #[test]
    fn invalid_or_excessively_nested_json_is_rejected() {
        let deeply_nested = [b"[".repeat(200), b"0".to_vec(), b"]".repeat(200)].concat();
        for json in [b"{}{}".to_vec(), deeply_nested, b"NaN".to_vec(), vec![0xff]] {
            let file = write_envelope(schema::JSON, &json, 0).expect("write");
            assert!(matches!(Reader::default().decode(&mut file.clone()), Err(Error::Json(_))));
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn payload_is_always_masked_and_flags_are_reserved() {
        let file = write_bytes(b"hello", 0).expect("write");
        assert_eq!(file.get(5), Some(&0));
        assert_ne!(file.get(20..).expect("payload"), b"hello");
        assert_eq!(
            Reader::default().read_envelope(&mut file.clone()).expect("read").payload,
            b"hello"
        );
    }

    #[cfg(feature = "write")]
    #[test]
    fn every_reserved_flag_is_rejected() {
        let mut file = write_bytes(b"hello", 42).expect("write");
        for flags in 1..=u8::MAX {
            *file.get_mut(5).expect("flags byte") = flags;
            assert!(
                matches!(Reader::default().decode(&mut file.clone()), Err(Error::UnsupportedFlags(actual)) if actual == flags)
            );
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn plaintext_cannot_bypass_the_mandatory_mask() {
        let mut file = write_bytes(b"hello", 0).expect("write");
        file.get_mut(20..).expect("payload").copy_from_slice(b"hello");
        assert!(matches!(
            Reader::default().decode(&mut file.clone()),
            Err(Error::ChecksumMismatch)
        ));
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
                Reader::default().decode(&mut file.clone()).expect_err("invalid file").to_string(),
                expected
            );
        }
        let mut trailing = original;
        trailing.push(0);
        assert!(matches!(Reader::default().decode(&mut trailing), Err(Error::LengthMismatch)));
    }
}
