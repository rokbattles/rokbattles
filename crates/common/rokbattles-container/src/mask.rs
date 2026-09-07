//! Reversible payload transforms. Each stage starts from the same public seed;
//! decoding reverses both the stage order and each stage's byte operations.

#[cfg(feature = "write")]
pub(crate) fn encode(payload: &mut [u8], seed: u32) {
    xor_mask(payload, seed);
    layered(payload, seed, false);
}

#[cfg(feature = "read")]
pub(crate) fn decode(payload: &mut [u8], seed: u32) {
    layered(payload, seed, true);
    xor_mask(payload, seed);
}

fn xor_mask(payload: &mut [u8], seed: u32) {
    // Zero is a valid public seed; use a fixed nonzero state to avoid a no-op mask.
    let mut state = if seed == 0 { 0x6d2b_79f5 } else { seed };
    for chunk in payload.chunks_mut(4) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        for (byte, mask) in chunk.iter_mut().zip(state.to_le_bytes()) {
            *byte ^= mask;
        }
    }
}

fn next(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn layered(payload: &mut [u8], seed: u32, decode: bool) {
    // Reset at the stage boundary, then carry state across blocks. Changing
    // either boundary changes the wire format, including partial final blocks.
    let mut state = if seed == 0 { 0x6d2b_79f5 } else { seed };
    for block in payload.chunks_mut(32) {
        let mut keys = [0_u32; 32];
        for key in keys.iter_mut().take(block.len()) {
            *key = next(&mut state);
        }
        // Generate the same Fisher-Yates schedule before transforming bytes.
        // Decode applies swaps in reverse order, restoring the feedback order.
        let mut swaps = [0_usize; 32];
        for (index, slot) in swaps.iter_mut().enumerate().take(block.len()).skip(1).rev() {
            *slot = next(&mut state) as usize % (index + 1);
        }
        if decode {
            for (index, &other) in swaps.iter().enumerate().take(block.len()).skip(1) {
                block.swap(index, other);
            }
        }
        let mut previous = 0;
        for (byte, key) in block.iter_mut().zip(keys) {
            let [add, xor, rotation, _] = key.to_le_bytes();
            let rotation = u32::from(rotation & 7);
            if decode {
                // Feedback uses the preceding encoded byte, not its decoded value.
                let encoded = *byte;
                *byte = (*byte ^ xor ^ previous).rotate_right(rotation).wrapping_sub(add);
                previous = encoded;
            } else {
                *byte = byte.wrapping_add(add).rotate_left(rotation) ^ xor ^ previous;
                previous = *byte;
            }
        }
        if !decode {
            for (index, &other) in swaps.iter().enumerate().take(block.len()).skip(1).rev() {
                block.swap(index, other);
            }
        }
    }
}

#[cfg(all(test, feature = "read", feature = "write"))]
mod tests {
    use super::*;

    #[test]
    fn round_trips_partial_words_and_blocks() {
        for length in 0..513 {
            let original: Vec<u8> = (0..=255).cycle().take(length).collect();
            for seed in [0, 1, u32::MAX] {
                let mut payload = original.clone();
                encode(&mut payload, seed);
                decode(&mut payload, seed);
                assert_eq!(payload, original);
            }
        }
    }
}
