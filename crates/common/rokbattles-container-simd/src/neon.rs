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
            let keys = std::array::from_fn(|index| schedule[index][lane]);
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
    let mut add = [0_u8; 32];
    let mut xor = [0_u8; 32];
    let mut rotation = [0_u8; 32];
    for (((add, xor), rotation), key) in add.iter_mut().zip(&mut xor).zip(&mut rotation).zip(keys) {
        let bytes = key.to_le_bytes();
        *add = bytes[0];
        *xor = bytes[1];
        *rotation = bytes[2] & 7;
    }
    let mut previous = [0_u8; 32];
    // Capture feedback before either half is modified, including the byte that
    // crosses the 16-byte vector boundary. Feedback resets only every 32 bytes.
    previous[1..].copy_from_slice(&block[..31]);
    for ((((block, add), xor), rotation), previous) in block
        .as_chunks_mut::<16>()
        .0
        .iter_mut()
        .zip(add.as_chunks::<16>().0)
        .zip(xor.as_chunks::<16>().0)
        .zip(rotation.as_chunks::<16>().0)
        .zip(previous.as_chunks::<16>().0)
    {
        // SAFETY: Each input contains 16 initialized bytes, and NEON is enabled.
        // The intrinsic permits an unaligned address.
        let encoded = unsafe { vld1q_u8(block.as_ptr()) };
        // SAFETY: The XOR chunk contains 16 initialized bytes.
        let xors = unsafe { vld1q_u8(xor.as_ptr()) };
        // SAFETY: The feedback chunk contains 16 initialized bytes.
        let feedback = unsafe { vld1q_u8(previous.as_ptr()) };
        // SAFETY: The rotation chunk contains 16 initialized bytes.
        let rotations = unsafe { vld1q_u8(rotation.as_ptr()) };
        // SAFETY: The addition chunk contains 16 initialized bytes.
        let additions = unsafe { vld1q_u8(add.as_ptr()) };
        let value = veorq_u8(veorq_u8(encoded, xors), feedback);
        let rotations = vreinterpretq_s8_u8(rotations);
        // Signed counts select the shift direction. For a zero rotation, the
        // right part is unchanged and the left shift by eight contributes zero.
        let right = vshlq_u8(value, vnegq_s8(rotations));
        let left = vshlq_u8(value, vsubq_s8(vdupq_n_s8(8), rotations));
        let value = vsubq_u8(vorrq_u8(right, left), additions);
        // SAFETY: The destination contains writable storage for all 16 bytes.
        unsafe { vst1q_u8(block.as_mut_ptr(), value) };
    }
}
