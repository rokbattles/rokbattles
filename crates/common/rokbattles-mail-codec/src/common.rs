//! Shared wire-format constants and checksum calculation.

pub(crate) const TAG_BOOL: u8 = 0x01;
pub(crate) const TAG_F64: u8 = 0x03;
pub(crate) const TAG_STRING: u8 = 0x04;
pub(crate) const TAG_TABLE: u8 = 0x05;
pub(crate) const TABLE_END: u8 = 0xff;

pub(crate) const FILE_MARKER: u8 = 0xff;
pub(crate) const FILE_HEADER_LEN: usize = 9;
pub(crate) const CHECKSUM_SEED: u64 = 0x1505;

pub(crate) const MAX_DEPTH: usize = 128;

pub(crate) fn file_checksum(buffer: &[u8]) -> u64 {
    const CHECKSUM_MULTIPLIER: u64 = 33;
    const ZEROED_CHECKSUM_BYTES_FACTOR: u64 = CHECKSUM_MULTIPLIER.pow(8);

    let Some((&marker, rest)) = buffer.split_first() else {
        return CHECKSUM_SEED;
    };
    let Some(payload) = rest.get(FILE_HEADER_LEN - 1..) else {
        return rest.iter().fold(
            CHECKSUM_SEED.wrapping_mul(CHECKSUM_MULTIPLIER).wrapping_add(u64::from(marker)),
            |hash, _| hash.wrapping_mul(CHECKSUM_MULTIPLIER),
        );
    };

    // Each zero byte advances DJB2 by a factor of 33. Multiplying by 33^8 skips the
    // checksum field without copying the file to zero those bytes.
    let header_hash = CHECKSUM_SEED
        .wrapping_mul(CHECKSUM_MULTIPLIER)
        .wrapping_add(u64::from(marker))
        .wrapping_mul(ZEROED_CHECKSUM_BYTES_FACTOR);
    rokbattles_djb2_simd::checksum(header_hash, payload)
}

#[cfg(test)]
mod tests {
    use super::file_checksum;

    #[test]
    fn checksum_matches_bytewise_wire_definition() {
        let buffer: Vec<u8> = (0..=255).collect();
        // Include short headers and payload sizes on either side of SIMD blocks.
        for length in 0..=buffer.len() {
            let input = buffer.get(..length).expect("prefix is in bounds");
            let expected = input.iter().enumerate().fold(5_381_u64, |hash, (offset, &byte)| {
                let byte = if (1..9).contains(&offset) { 0 } else { byte };
                hash.wrapping_mul(33).wrapping_add(u64::from(byte))
            });
            assert_eq!(file_checksum(input), expected, "buffer length {length}");
        }
    }
}
