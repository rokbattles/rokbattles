pub(crate) const MULTIPLIER: u64 = 33;

// Expanding four DJB2 rounds gives
// `h' = h * 33^4 + b0 * 33^3 + b1 * 33^2 + b2 * 33 + b3`.
// Its largest byte contribution is 9_450_300, so the SIMD reductions fit in `u32` lanes.
const MULTIPLIER_2: u64 = MULTIPLIER * MULTIPLIER;
const MULTIPLIER_3: u64 = MULTIPLIER_2 * MULTIPLIER;
pub(crate) const MULTIPLIER_4: u64 = MULTIPLIER_3 * MULTIPLIER;

pub(crate) fn checksum(mut hash: u64, bytes: &[u8]) -> u64 {
    // Use the same four-round expansion as the SIMD implementations, then apply the original
    // recurrence to the tail.
    let (chunks, remainder) = bytes.as_chunks::<4>();
    for &[byte_0, byte_1, byte_2, byte_3] in chunks {
        let contribution = u64::from(byte_0) * MULTIPLIER_3
            + u64::from(byte_1) * MULTIPLIER_2
            + u64::from(byte_2) * MULTIPLIER
            + u64::from(byte_3);
        hash = hash.wrapping_mul(MULTIPLIER_4).wrapping_add(contribution);
    }
    for &byte in remainder {
        hash = hash.wrapping_mul(MULTIPLIER).wrapping_add(u64::from(byte));
    }
    hash
}
