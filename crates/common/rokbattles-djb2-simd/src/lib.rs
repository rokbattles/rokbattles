//! A runtime-dispatched implementation of 64-bit wrapping DJB2.
//!
//! This crate advances a caller-provided hash state over a byte slice. On `x86_64` it uses AVX2
//! when the running CPU supports it, and on `aarch64` it uses NEON when available. All other cases
//! use the equivalent portable scalar implementation.
//!
//! DJB2 is a non-cryptographic hash. Do not rely on it for collision resistance or other security
//! properties.
//!
//! # Example
//!
//! ```
//! let hash = rokbattles_djb2_simd::checksum(5_381, b"hello");
//!
//! assert_eq!(hash, 210_714_636_441);
//! ```

#![deny(unsafe_op_in_unsafe_fn)]

mod scalar;

#[cfg(test)]
use scalar::MULTIPLIER;
use scalar::{MULTIPLIER_4, checksum as checksum_scalar};

/// Extends `hash` with `bytes` using 64-bit wrapping DJB2.
///
/// For each byte `b`, the hash state is updated as `hash = hash * 33 + b`, modulo `2^64`. The
/// caller supplies the initial state; use `5_381` for the conventional DJB2 seed. Supplying an
/// empty slice returns `hash` unchanged.
///
/// Calls can be chained to hash data in separate chunks. Runtime SIMD dispatch is transparent:
/// every supported implementation produces the same result as the scalar recurrence.
///
/// # Example
///
/// ```
/// let first = rokbattles_djb2_simd::checksum(5_381, b"hel");
/// let chunked = rokbattles_djb2_simd::checksum(first, b"lo");
/// let contiguous = rokbattles_djb2_simd::checksum(5_381, b"hello");
///
/// assert_eq!(chunked, contiguous);
/// ```
#[must_use]
pub fn checksum(hash: u64, bytes: &[u8]) -> u64 {
    platform::checksum(hash, bytes)
}

#[cfg(target_arch = "aarch64")]
mod platform {
    use std::arch::aarch64::{
        uint16x4_t, vget_high_u8, vget_high_u16, vget_low_u8, vget_low_u16, vld1_u16, vld1q_u8,
        vmovl_u8, vmull_u16, vpaddq_u32, vst1q_u32,
    };

    use super::{MULTIPLIER_4, checksum_scalar};

    // Coefficients for a four-byte expansion, ordered from the oldest byte to the newest.
    const WEIGHTS: [u16; 4] = [35_937, 1_089, 33, 1];

    pub(super) fn checksum(hash: u64, bytes: &[u8]) -> u64 {
        if std::arch::is_aarch64_feature_detected!("neon") {
            // SAFETY: Runtime detection satisfies `checksum_neon`'s NEON requirement. Its memory
            // operations derive their bounds from `bytes` and local arrays.
            unsafe { checksum_neon(hash, bytes) }
        } else {
            checksum_scalar(hash, bytes)
        }
    }

    #[target_feature(enable = "neon")]
    unsafe fn checksum_neon(mut hash: u64, bytes: &[u8]) -> u64 {
        // SAFETY: `WEIGHTS` contains the four initialized `u16` values read by `vld1_u16`.
        let weights: uint16x4_t = unsafe { vld1_u16(WEIGHTS.as_ptr()) };
        let (chunks, remainder) = bytes.as_chunks::<16>();
        for chunk in chunks {
            // SAFETY: Each `chunk` contains the 16 initialized bytes read by `vld1q_u8`; the
            // intrinsic permits an unaligned address.
            let input = unsafe { vld1q_u8(chunk.as_ptr()) };

            // Multiply four consecutive four-byte groups by the expansion coefficients, then
            // reduce each group to one contribution. The resulting lanes remain in input order.
            let low = vmovl_u8(vget_low_u8(input));
            let high = vmovl_u8(vget_high_u8(input));
            let products_0 = vmull_u16(vget_low_u16(low), weights);
            let products_1 = vmull_u16(vget_high_u16(low), weights);
            let products_2 = vmull_u16(vget_low_u16(high), weights);
            let products_3 = vmull_u16(vget_high_u16(high), weights);
            let pairs_01 = vpaddq_u32(products_0, products_1);
            let pairs_23 = vpaddq_u32(products_2, products_3);
            let sums = vpaddq_u32(pairs_01, pairs_23);
            let mut contributions = [0_u32; 4];
            // SAFETY: `contributions` provides writable storage for the four `u32` lanes written
            // by `vst1q_u32`.
            unsafe { vst1q_u32(contributions.as_mut_ptr(), sums) };

            for contribution in contributions {
                hash = hash.wrapping_mul(MULTIPLIER_4).wrapping_add(u64::from(contribution));
            }
        }

        checksum_scalar(hash, remainder)
    }
}

#[cfg(target_arch = "x86_64")]
mod platform {
    use std::arch::x86_64::{
        __m128i, __m256i, _mm_loadu_si128, _mm_srli_si128, _mm256_cvtepu8_epi32, _mm256_hadd_epi32,
        _mm256_mullo_epi32, _mm256_setr_epi32, _mm256_storeu_si256,
    };

    use super::{MULTIPLIER_4, checksum_scalar};

    pub(super) fn checksum(hash: u64, bytes: &[u8]) -> u64 {
        if std::arch::is_x86_feature_detected!("avx2") {
            // SAFETY: Runtime detection satisfies `checksum_avx2`'s AVX2 requirement. Its memory
            // operations derive their bounds from `bytes` and local arrays.
            unsafe { checksum_avx2(hash, bytes) }
        } else {
            checksum_scalar(hash, bytes)
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn checksum_avx2(mut hash: u64, bytes: &[u8]) -> u64 {
        // Repeat the four expansion coefficients in both 128-bit halves of the AVX2 vector.
        let weights = _mm256_setr_epi32(35_937, 1_089, 33, 1, 35_937, 1_089, 33, 1);
        let (chunks, remainder) = bytes.as_chunks::<16>();
        for chunk in chunks {
            // SAFETY: Each `chunk` contains the 16 initialized bytes read by `_mm_loadu_si128`;
            // the intrinsic permits an unaligned address.
            let input = unsafe { _mm_loadu_si128(chunk.as_ptr().cast::<__m128i>()) };
            let low = _mm256_cvtepu8_epi32(input);
            let high = _mm256_cvtepu8_epi32(_mm_srli_si128::<8>(input));
            let products_low = _mm256_mullo_epi32(low, weights);
            let products_high = _mm256_mullo_epi32(high, weights);
            let pairs = _mm256_hadd_epi32(products_low, products_high);
            let sums = _mm256_hadd_epi32(pairs, pairs);
            let mut lanes = [0_u32; 8];
            // SAFETY: `lanes` provides writable storage for the eight `u32` lanes written by
            // `_mm256_storeu_si256`; the intrinsic permits an unaligned address.
            unsafe { _mm256_storeu_si256(lanes.as_mut_ptr().cast::<__m256i>(), sums) };

            // Horizontal additions operate within each 128-bit half. After two reductions the
            // four input-order contributions occupy lanes 0, 4, 1, and 5, respectively.
            let [sum_0, sum_2, _, _, sum_1, sum_3, _, _] = lanes;
            for contribution in [sum_0, sum_1, sum_2, sum_3] {
                hash = hash.wrapping_mul(MULTIPLIER_4).wrapping_add(u64::from(contribution));
            }
        }

        checksum_scalar(hash, remainder)
    }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
mod platform {
    pub(super) fn checksum(hash: u64, bytes: &[u8]) -> u64 {
        super::checksum_scalar(hash, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep the oracle byte-at-a-time so it does not reproduce the optimized four-byte recurrence.
    fn checksum_reference(mut hash: u64, bytes: &[u8]) -> u64 {
        for &byte in bytes {
            hash = hash.wrapping_mul(MULTIPLIER).wrapping_add(u64::from(byte));
        }
        hash
    }

    #[test]
    fn checksum_matches_reference_for_varied_lengths_and_values() {
        // Because 197 is odd, this affine sequence visits every byte value before repeating. The
        // dense lengths exercise every scalar and SIMD tail; the larger cases straddle powers of
        // two and cover the complete input buffer.
        let bytes = (0..=u16::MAX)
            .map(|value| (value.wrapping_mul(197).wrapping_add(101) & 0xff) as u8)
            .collect::<Vec<_>>();

        let lengths = (0..=4096).chain([8191, 8192, 8193, 32_767, 65_535, 65_536]);
        for length in lengths {
            assert_eq!(
                checksum(0x1505, &bytes[..length]),
                checksum_reference(0x1505, &bytes[..length])
            );
        }
    }

    #[test]
    fn scalar_checksum_matches_reference_for_varied_lengths_and_values() {
        // Use the same varied input matrix while bypassing runtime dispatch, so the portable
        // implementation remains independently covered on SIMD-capable development machines.
        let bytes = (0..=u16::MAX)
            .map(|value| (value.wrapping_mul(197).wrapping_add(101) & 0xff) as u8)
            .collect::<Vec<_>>();

        let lengths = (0..=4096).chain([8191, 8192, 8193, 32_767, 65_535, 65_536]);
        for length in lengths {
            assert_eq!(
                checksum_scalar(0x1505, &bytes[..length]),
                checksum_reference(0x1505, &bytes[..length])
            );
        }
    }
}
