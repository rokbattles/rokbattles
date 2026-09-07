use std::arch::aarch64::*;

use crate::schedule::jump;

#[expect(
    clippy::indexing_slicing,
    reason = "Fixed batches bound lanes to 0..4 and schedule rows to 0..63"
)]
#[target_feature(enable = "neon")]
pub(super) unsafe fn decode_blocks(payload: &mut [u8], mut state: u32) -> (usize, u32) {
    let mut processed = 0;
    for batch in payload.as_chunks_mut::<128>().0 {
        let mut seeds = [0_u32; 4];
        for seed in &mut seeds {
            *seed = state;
            state = jump(state);
        }
        let mut schedule = [[0_u32; 4]; 63];
        // SAFETY: `seeds` contains four initialized u32s; the load permits an
        // unaligned address. NEON is required by this function.
        let mut states = unsafe { vld1q_u32(seeds.as_ptr()) };
        for row in &mut schedule {
            states = veorq_u32(states, vshlq_n_u32::<13>(states));
            states = veorq_u32(states, vshrq_n_u32::<17>(states));
            states = veorq_u32(states, vshlq_n_u32::<5>(states));
            // SAFETY: The row holds four initialized u32 lanes and permits an unaligned store.
            unsafe { vst1q_u32(row.as_mut_ptr(), states) };
        }
        for (lane, block) in batch.as_chunks_mut::<32>().0.iter_mut().enumerate() {
            let keys = std::array::from_fn(|index| schedule[index][lane].to_le());
            // Rows 32..63 hold choices for indices 31..1. Replay them in reverse.
            for index in 1..32 {
                block.swap(index, schedule[63 - index][lane] as usize % (index + 1));
            }
            // SAFETY: NEON is enabled; block and keys are complete initialized arrays.
            unsafe { transform(block, &keys) };
        }
        processed += batch.len();
    }
    (processed, state)
}

#[target_feature(enable = "neon")]
unsafe fn transform(block: &mut [u8; 32], keys: &[u32; 32]) {
    let mut previous = vdupq_n_u8(0);
    for (block, keys) in block.as_chunks_mut::<16>().0.iter_mut().zip(keys.as_chunks::<16>().0) {
        // Each little-endian key contains addition, XOR, rotation, and an unused byte.
        // The load separates these fields into four vectors, keeping the keys in order.
        // SAFETY: `keys` contains the 64 initialized bytes read by `vld4q_u8`;
        // the intrinsic permits an unaligned address. NEON is enabled.
        let fields = unsafe { vld4q_u8(keys.as_ptr().cast()) };
        // SAFETY: `block` contains the 16 initialized bytes read by `vld1q_u8`;
        // the intrinsic permits an unaligned address.
        let encoded = unsafe { vld1q_u8(block.as_ptr()) };
        // Each byte uses the preceding encoded byte as feedback. The first half
        // starts with zero; the second starts with the first half's last encoded byte.
        let feedback = vextq_u8::<15>(previous, encoded);
        previous = encoded;
        let value = veorq_u8(veorq_u8(encoded, fields.1), feedback);
        let rotations = vreinterpretq_s8_u8(vandq_u8(fields.2, vdupq_n_u8(7)));
        // Signed counts select the shift direction. For a zero rotation, the
        // right part is unchanged and the left shift by eight contributes zero.
        let right = vshlq_u8(value, vnegq_s8(rotations));
        let left = vshlq_u8(value, vsubq_s8(vdupq_n_s8(8), rotations));
        let value = vsubq_u8(vorrq_u8(right, left), fields.0);
        // SAFETY: The destination contains writable storage for all 16 bytes.
        unsafe { vst1q_u8(block.as_mut_ptr(), value) };
    }
}
