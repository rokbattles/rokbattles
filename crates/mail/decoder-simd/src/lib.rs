//! Runtime-dispatched SIMD and portable scalar checksum implementations for the mail decoder.

#![deny(unsafe_op_in_unsafe_fn)]

const MULTIPLIER: u64 = 33;
const MULTIPLIER_2: u64 = MULTIPLIER * MULTIPLIER;
const MULTIPLIER_3: u64 = MULTIPLIER_2 * MULTIPLIER;
const MULTIPLIER_4: u64 = MULTIPLIER_3 * MULTIPLIER;

/// Extend a wrapping DJB2 checksum with `bytes`, using an available SIMD implementation or the
/// equivalent scalar fallback.
#[must_use]
pub fn checksum(hash: u64, bytes: &[u8]) -> u64 {
    platform::checksum(hash, bytes)
}

fn checksum_scalar(mut hash: u64, bytes: &[u8]) -> u64 {
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        if let &[byte_0, byte_1, byte_2, byte_3] = chunk {
            let contribution = u64::from(byte_0) * MULTIPLIER_3
                + u64::from(byte_1) * MULTIPLIER_2
                + u64::from(byte_2) * MULTIPLIER
                + u64::from(byte_3);
            hash = hash.wrapping_mul(MULTIPLIER_4).wrapping_add(contribution);
        }
    }
    for &byte in chunks.remainder() {
        hash = hash.wrapping_mul(MULTIPLIER).wrapping_add(u64::from(byte));
    }
    hash
}

#[cfg(target_arch = "aarch64")]
mod platform {
    use std::arch::aarch64::{
        uint16x4_t, vget_high_u8, vget_high_u16, vget_low_u8, vget_low_u16, vld1_u16, vld1q_u8,
        vmovl_u8, vmull_u16, vpaddq_u32, vst1q_u32,
    };

    use super::{MULTIPLIER_4, checksum_scalar};

    const WEIGHTS: [u16; 4] = [35_937, 1_089, 33, 1];

    pub(super) fn checksum(hash: u64, bytes: &[u8]) -> u64 {
        if std::arch::is_aarch64_feature_detected!("neon") {
            // SAFETY: Runtime detection guarantees NEON is available. The implementation only
            // performs unaligned reads within `bytes` and writes to a local array.
            unsafe { checksum_neon(hash, bytes) }
        } else {
            checksum_scalar(hash, bytes)
        }
    }

    #[target_feature(enable = "neon")]
    unsafe fn checksum_neon(mut hash: u64, bytes: &[u8]) -> u64 {
        // SAFETY: `WEIGHTS.as_ptr()` points to four contiguous initialized `u16` values, matching
        // the load width.
        let weights: uint16x4_t = unsafe { vld1_u16(WEIGHTS.as_ptr()) };
        let mut chunks = bytes.chunks_exact(16);
        for chunk in &mut chunks {
            // SAFETY: `chunks_exact(16)` guarantees 16 readable bytes.
            let input = unsafe { vld1q_u8(chunk.as_ptr()) };
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
            // SAFETY: `contributions` has space for all four lanes.
            unsafe { vst1q_u32(contributions.as_mut_ptr(), sums) };

            for contribution in contributions {
                hash = hash.wrapping_mul(MULTIPLIER_4).wrapping_add(u64::from(contribution));
            }
        }

        checksum_scalar(hash, chunks.remainder())
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
            // SAFETY: Runtime detection guarantees AVX2 is available. The implementation only
            // performs unaligned reads within `bytes` and writes to a local array.
            unsafe { checksum_avx2(hash, bytes) }
        } else {
            checksum_scalar(hash, bytes)
        }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn checksum_avx2(mut hash: u64, bytes: &[u8]) -> u64 {
        let weights = _mm256_setr_epi32(35_937, 1_089, 33, 1, 35_937, 1_089, 33, 1);
        let mut chunks = bytes.chunks_exact(16);
        for chunk in &mut chunks {
            // SAFETY: `chunks_exact(16)` guarantees 16 readable bytes.
            let input = unsafe { _mm_loadu_si128(chunk.as_ptr().cast::<__m128i>()) };
            let low = _mm256_cvtepu8_epi32(input);
            let high = _mm256_cvtepu8_epi32(_mm_srli_si128::<8>(input));
            let products_low = _mm256_mullo_epi32(low, weights);
            let products_high = _mm256_mullo_epi32(high, weights);
            let pairs = _mm256_hadd_epi32(products_low, products_high);
            let sums = _mm256_hadd_epi32(pairs, pairs);
            let mut lanes = [0_u32; 8];
            // SAFETY: `lanes` has space for all eight lanes.
            unsafe { _mm256_storeu_si256(lanes.as_mut_ptr().cast::<__m256i>(), sums) };

            let [sum_0, sum_2, _, _, sum_1, sum_3, _, _] = lanes;
            for contribution in [sum_0, sum_1, sum_2, sum_3] {
                hash = hash.wrapping_mul(MULTIPLIER_4).wrapping_add(u64::from(contribution));
            }
        }

        checksum_scalar(hash, chunks.remainder())
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

    fn checksum_reference(mut hash: u64, bytes: &[u8]) -> u64 {
        for &byte in bytes {
            hash = hash.wrapping_mul(MULTIPLIER).wrapping_add(u64::from(byte));
        }
        hash
    }

    #[test]
    fn checksum_matches_reference_for_varied_lengths_and_values() {
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
