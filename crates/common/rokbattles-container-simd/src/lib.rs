//! SIMD decoding for the layered mask in ROKB v1 files.
//!
//! AVX2 processes eight 32-byte blocks per batch; NEON processes four.
//! The caller decodes the remaining bytes with the scalar implementation, then
//! removes the XOR mask. Header and checksum validation belong to the container reader.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_arch = "x86_64")]
mod avx2;
#[cfg(target_arch = "aarch64")]
mod neon;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
mod schedule;

/// Decodes a prefix of complete layered-mask blocks and advances its PRNG state.
///
/// Returns `(bytes_processed, next_state)` and modifies only that prefix.
/// Continue scalar decoding on the remainder with `next_state`. The processed
/// byte count is always a multiple of 32.
///
/// `state` is the current xorshift32 state. At the start of a payload, pass its
/// seed, replacing zero with `0x6d2b79f5`. This function does not normalize it.
///
/// Returns `(0, state)` when no supported backend is available or fewer than
/// one batch remains: 256 bytes for AVX2, 128 bytes for NEON.
/// CPU detection precedes every call into target-specific code.
pub fn decode_blocks(payload: &mut [u8], state: u32) -> (usize, u32) {
    if payload.len() < 128 {
        return (0, state);
    }
    #[cfg(target_arch = "x86_64")]
    if payload.len() >= 256 && std::arch::is_x86_feature_detected!("avx2") {
        // SAFETY: AVX2 was detected. The backend bounds all loads and stores to
        // initialized fixed-size arrays and complete 256-byte input batches.
        return unsafe { avx2::decode_blocks(payload, state) };
    }
    #[cfg(target_arch = "aarch64")]
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: NEON was detected. The backend bounds memory operations to
        // initialized fixed-size arrays and complete 128-byte input batches.
        return unsafe { neon::decode_blocks(payload, state) };
    }
    (0, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn next(state: &mut u32) -> u32 {
        *state ^= *state << 13;
        *state ^= *state >> 17;
        *state ^= *state << 5;
        *state
    }

    fn scalar(payload: &mut [u8], mut state: u32) -> u32 {
        for block in payload.as_chunks_mut::<32>().0 {
            let keys: [u32; 32] = std::array::from_fn(|_| next(&mut state));
            let mut swaps: Vec<_> = (1..32)
                .rev()
                .map(|index| (index, next(&mut state) as usize % (index + 1)))
                .collect();
            swaps.reverse();
            for (index, other) in swaps {
                block.swap(index, other);
            }
            let mut previous = 0;
            for (byte, key) in block.iter_mut().zip(keys) {
                let [add, xor, rotation, _] = key.to_le_bytes();
                let encoded = *byte;
                *byte = (*byte ^ xor ^ previous)
                    .rotate_right(u32::from(rotation & 7))
                    .wrapping_sub(add);
                previous = encoded;
            }
        }
        state
    }

    #[test]
    fn matches_scalar_state_and_bytes_without_touching_remainder_or_guards() {
        for length in 0..1_025 {
            for offset in [0, 1, 7, 15, 31] {
                for seed in [0, 1, 42, u32::MAX] {
                    let mut bytes: Vec<u8> =
                        (0..length + offset + 32).map(|i| (i * 197) as u8).collect();
                    let mut expected = bytes.clone();
                    let payload = bytes.get_mut(offset..offset + length).expect("payload");
                    let (processed, state) = decode_blocks(payload, seed);
                    assert!(processed <= length);
                    assert_eq!(processed % 32, 0);
                    let expected_state =
                        scalar(expected.get_mut(offset..offset + processed).expect("prefix"), seed);
                    assert_eq!(state, expected_state);
                    assert_eq!(bytes, expected);
                    #[cfg(target_arch = "x86_64")]
                    if std::arch::is_x86_feature_detected!("avx2") {
                        assert_eq!(processed, length / 256 * 256);
                    } else {
                        assert_eq!(processed, 0);
                    }
                    #[cfg(target_arch = "aarch64")]
                    if std::arch::is_aarch64_feature_detected!("neon") {
                        assert_eq!(processed, length / 128 * 128);
                    } else {
                        assert_eq!(processed, 0);
                    }
                    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
                    assert_eq!(processed, 0);
                }
            }
        }
    }
}
