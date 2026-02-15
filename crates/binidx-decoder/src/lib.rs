#![forbid(unsafe_code)]

/// A parsed string entry in a decoded table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEntry {
    /// 1-based entry index.
    pub index: usize,
    /// Offset of the entry's `u16` length field in the decoded table.
    pub offset: usize,
    /// UTF-8 text value.
    pub value: String,
}

/// Parsed decoded table metadata and entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringTable {
    /// Raw 2-byte header found at the start of the decoded table.
    pub header: [u8; 2],
    /// Parsed string entries.
    pub entries: Vec<StringEntry>,
    /// Unparsed trailing bytes after the last valid entry.
    pub trailer: Vec<u8>,
}

/// Key/value row after pairing index and value tables by position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// 1-based row index.
    pub index: usize,
    /// Key from the index table.
    pub key: String,
    /// Text from the value table.
    pub value: String,
}

/// Decoded and paired data from `*.idx` + `*.bin` tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringCatalog {
    /// Parsed index table.
    pub keys: StringTable,
    /// Parsed value table.
    pub values: StringTable,
    /// Positionally paired rows (`min(keys, values)`).
    pub rows: Vec<CatalogEntry>,
    /// Keys without a matching value row.
    pub extra_keys: Vec<StringEntry>,
    /// Values without a matching key row.
    pub extra_values: Vec<StringEntry>,
}

/// Errors produced while decoding or parsing tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Table was too short to contain the 2-byte header.
    UnexpectedEof { len: usize },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnexpectedEof { len } => {
                write!(f, "table is too short (need at least 2 bytes, got {len})")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Apply XOR decoding to a byte slice.
pub fn xor_decode(input: &[u8], key: u8) -> Vec<u8> {
    input.iter().map(|byte| byte ^ key).collect()
}

/// Parse a decoded byte buffer as a `[u16_le length][utf8 bytes]...` table.
///
/// The parser assumes the first 2 bytes are a table header. It then reads
/// variable entries until the next length would exceed remaining bytes.
///
/// If UTF-8 decoding fails for an entry, parsing stops and the remaining bytes
/// are stored in [`StringTable::trailer`] instead of returning an error.
pub fn parse_table(decoded: &[u8]) -> Result<StringTable, DecodeError> {
    if decoded.len() < 2 {
        return Err(DecodeError::UnexpectedEof { len: decoded.len() });
    }

    let header = [decoded[0], decoded[1]];
    let mut entries = Vec::new();
    let mut offset = 2usize;

    while offset + 2 <= decoded.len() {
        let length_offset = offset;
        let length = u16::from_le_bytes([decoded[offset], decoded[offset + 1]]) as usize;
        offset += 2;

        if length == 0 || offset + length > decoded.len() {
            offset = length_offset;
            break;
        }

        let raw = &decoded[offset..offset + length];
        let value = match std::str::from_utf8(raw) {
            Ok(value) => value.to_owned(),
            Err(_) => {
                offset = length_offset;
                break;
            }
        };

        entries.push(StringEntry {
            index: entries.len() + 1,
            offset: length_offset,
            value,
        });
        offset += length;
    }

    Ok(StringTable {
        header,
        entries,
        trailer: decoded[offset..].to_vec(),
    })
}

/// Build a key/value catalog by pairing decoded index/value tables by position.
pub fn decode_catalog_from_decoded_tables(
    idx_decoded: &[u8],
    bin_decoded: &[u8],
) -> Result<StringCatalog, DecodeError> {
    let keys = parse_table(idx_decoded)?;
    let values = parse_table(bin_decoded)?;

    let matched_len = keys.entries.len().min(values.entries.len());
    let rows = (0..matched_len)
        .map(|idx| CatalogEntry {
            index: idx + 1,
            key: keys.entries[idx].value.clone(),
            value: values.entries[idx].value.clone(),
        })
        .collect::<Vec<_>>();

    Ok(StringCatalog {
        extra_keys: keys.entries[matched_len..].to_vec(),
        extra_values: values.entries[matched_len..].to_vec(),
        keys,
        values,
        rows,
    })
}

/// XOR decode encoded index/value tables and pair rows by position.
pub fn decode_catalog_from_encoded_tables(
    idx_encoded: &[u8],
    bin_encoded: &[u8],
    xor_key: u8,
) -> Result<StringCatalog, DecodeError> {
    let idx_decoded = xor_decode(idx_encoded, xor_key);
    let bin_decoded = xor_decode(bin_encoded, xor_key);
    decode_catalog_from_decoded_tables(&idx_decoded, &bin_decoded)
}

#[cfg(test)]
mod tests {
    use super::{
        DecodeError, decode_catalog_from_decoded_tables, decode_catalog_from_encoded_tables,
        parse_table, xor_decode,
    };

    fn table_with_strings(strings: &[&str]) -> Vec<u8> {
        let mut bytes = vec![0x78, 0x00];
        for value in strings {
            let raw = value.as_bytes();
            bytes.extend_from_slice(&(raw.len() as u16).to_le_bytes());
            bytes.extend_from_slice(raw);
        }
        bytes
    }

    #[test]
    fn xor_decode_works_with_synthetic_bytes() {
        let input = [0x10u8, 0x20, 0x30];
        let key = 0x5a;
        assert_eq!(xor_decode(&input, key), vec![0x4a, 0x7a, 0x6a]);
    }

    #[test]
    fn parse_table_reads_valid_entries() {
        let decoded = table_with_strings(&["N_1", "N_2"]);
        let table = parse_table(&decoded).expect("parse table");

        assert_eq!(table.header, [0x78, 0x00]);
        assert_eq!(table.entries.len(), 2);
        assert_eq!(table.entries[0].index, 1);
        assert_eq!(table.entries[0].offset, 2);
        assert_eq!(table.entries[0].value, "N_1");
        assert!(table.trailer.is_empty());
    }

    #[test]
    fn parse_table_stops_on_malformed_trailing_length() {
        let mut decoded = table_with_strings(&["N_1"]);
        decoded.extend_from_slice(&5u16.to_le_bytes());
        decoded.extend_from_slice(b"ab");

        let table = parse_table(&decoded).expect("parse table");
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.trailer, vec![5, 0, b'a', b'b']);
    }

    #[test]
    fn parse_table_stops_on_invalid_utf8_and_keeps_trailer() {
        let mut decoded = table_with_strings(&["N_1"]);
        decoded.extend_from_slice(&2u16.to_le_bytes());
        decoded.extend_from_slice(&[0xff, 0xff]);

        let table = parse_table(&decoded).expect("parse table");
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.trailer, vec![2, 0, 0xff, 0xff]);
    }

    #[test]
    fn parse_table_returns_eof_for_short_buffers() {
        let error = parse_table(&[0x78]).expect_err("must fail");
        assert_eq!(error, DecodeError::UnexpectedEof { len: 1 });
    }

    #[test]
    fn decode_catalog_pairs_rows_and_tracks_extra_keys() {
        let idx = table_with_strings(&["N_1", "N_2", "N_3"]);
        let bin = table_with_strings(&["Sword", "Shield"]);
        let catalog = decode_catalog_from_decoded_tables(&idx, &bin).expect("decode catalog");

        assert_eq!(catalog.rows.len(), 2);
        assert_eq!(catalog.rows[1].key, "N_2");
        assert_eq!(catalog.rows[1].value, "Shield");
        assert_eq!(catalog.extra_keys.len(), 1);
        assert_eq!(catalog.extra_keys[0].value, "N_3");
        assert!(catalog.extra_values.is_empty());
    }

    #[test]
    fn decode_catalog_pairs_rows_and_tracks_extra_values() {
        let idx = table_with_strings(&["N_1"]);
        let bin = table_with_strings(&["Sword", "Shield"]);
        let catalog = decode_catalog_from_decoded_tables(&idx, &bin).expect("decode catalog");

        assert_eq!(catalog.rows.len(), 1);
        assert!(catalog.extra_keys.is_empty());
        assert_eq!(catalog.extra_values.len(), 1);
        assert_eq!(catalog.extra_values[0].value, "Shield");
    }

    #[test]
    fn decode_catalog_from_encoded_tables_rounds_with_synthetic_bytes() {
        let key = 0x32;
        let idx_decoded = table_with_strings(&["N_1", "N_2"]);
        let bin_decoded = table_with_strings(&["Sword", "Shield"]);

        let idx_encoded = xor_decode(&idx_decoded, key);
        let bin_encoded = xor_decode(&bin_decoded, key);
        let catalog = decode_catalog_from_encoded_tables(&idx_encoded, &bin_encoded, key)
            .expect("decode catalog");

        assert_eq!(catalog.rows.len(), 2);
        assert_eq!(catalog.rows[0].key, "N_1");
        assert_eq!(catalog.rows[0].value, "Sword");
    }
}
