use std::arch::x86_64::*;

use crate::schedule::jump;

#[expect(
    clippy::indexing_slicing,
    reason = "Fixed batches bound lanes to 0..8 and schedule rows to 0..63"
)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn decode_blocks(payload: &mut [u8], mut state: u32) -> (usize, u32) {
    let mut processed = 0;
    for batch in payload.as_chunks_mut::<256>().0 {
        let mut seeds = [0_u32; 8];
        for seed in &mut seeds {
            *seed = state;
            state = jump(state);
        }
        let mut schedule = [[0_u32; 8]; 63];
        // SAFETY: `seeds` contains eight initialized u32s; the load permits an
        // unaligned address. AVX2 is required by this function.
        let mut states = unsafe { _mm256_loadu_si256(seeds.as_ptr().cast()) };
        for row in &mut schedule {
            states = _mm256_xor_si256(states, _mm256_slli_epi32::<13>(states));
            states = _mm256_xor_si256(states, _mm256_srli_epi32::<17>(states));
            states = _mm256_xor_si256(states, _mm256_slli_epi32::<5>(states));
            // SAFETY: The row holds eight initialized u32 lanes and permits an unaligned store.
            unsafe { _mm256_storeu_si256(row.as_mut_ptr().cast(), states) };
        }
        for (lane, block) in batch.as_chunks_mut::<32>().0.iter_mut().enumerate() {
            let keys = std::array::from_fn(|index| schedule[index][lane]);
            // Rows 32..63 hold choices for indices 31..1. Replay them in reverse.
            for index in 1..32 {
                block.swap(index, schedule[63 - index][lane] as usize % (index + 1));
            }
            // SAFETY: AVX2 is enabled; block and keys are complete initialized arrays.
            unsafe { transform(block, &keys) };
        }
        processed += batch.len();
    }
    (processed, state)
}

#[target_feature(enable = "avx2")]
unsafe fn transform(block: &mut [u8; 32], keys: &[u32; 32]) {
    let mut add = [0_u8; 32];
    let mut xor = [0_u8; 32];
    let mut rotation = [0_u8; 32];
    for (((add, xor), rotation), key) in add.iter_mut().zip(&mut xor).zip(&mut rotation).zip(keys) {
        let bytes = key.to_le_bytes();
        *add = bytes[0];
        *xor = bytes[1];
        *rotation = bytes[2];
    }
    let mut previous = [0_u8; 32];
    previous[1..].copy_from_slice(&block[..31]);
    // SAFETY: Each input contains exactly 32 initialized bytes. AVX2 is enabled,
    // and the intrinsic permits unaligned loads.
    let encoded = unsafe { _mm256_loadu_si256(block.as_ptr().cast()) };
    // SAFETY: The complete 32-byte XOR array is initialized.
    let xors = unsafe { _mm256_loadu_si256(xor.as_ptr().cast()) };
    // SAFETY: The complete 32-byte feedback array is initialized.
    let feedback = unsafe { _mm256_loadu_si256(previous.as_ptr().cast()) };
    // SAFETY: The complete 32-byte rotation array is initialized.
    let rotations = unsafe { _mm256_loadu_si256(rotation.as_ptr().cast()) };
    // SAFETY: The complete 32-byte addition array is initialized.
    let additions = unsafe { _mm256_loadu_si256(add.as_ptr().cast()) };
    let mut value = _mm256_xor_si256(_mm256_xor_si256(encoded, xors), feedback);
    // AVX2 has no per-byte variable rotate. Select rotations by 1, 2, and
    // 4 bits; masks remove the neighboring-byte bits from 16-bit shifts.
    macro_rules! rotate {
        ($right:literal, $left:literal, $mask:literal) => {{
            let low = _mm256_and_si256(_mm256_srli_epi16::<$right>(value), _mm256_set1_epi8($mask));
            let high =
                _mm256_and_si256(_mm256_slli_epi16::<$left>(value), _mm256_set1_epi8(!$mask));
            let bit = _mm256_set1_epi8($right);
            let select = _mm256_cmpeq_epi8(_mm256_and_si256(rotations, bit), bit);
            value = _mm256_blendv_epi8(value, _mm256_or_si256(low, high), select);
        }};
    }
    rotate!(1, 7, 127);
    rotate!(2, 6, 63);
    rotate!(4, 4, 15);
    value = _mm256_sub_epi8(value, additions);
    // SAFETY: The destination has writable storage for all 32 bytes.
    unsafe { _mm256_storeu_si256(block.as_mut_ptr().cast(), value) };
}
