//! A full block consumes 32 byte keys and 31 Fisher-Yates choices. Jumping the
//! linear xorshift32 recurrence by 63 steps gives independent starting states
//! for adjacent blocks without generating their keys serially.

const fn advance(mut state: u32) -> u32 {
    let mut step = 0;
    while step < 63 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        step += 1;
    }
    state
}

#[expect(clippy::indexing_slicing, reason = "Const loops stay within the four 256-entry tables")]
const fn tables() -> [[u32; 256]; 4] {
    let mut tables = [[0; 256]; 4];
    let mut byte = 0;
    while byte < 4 {
        let mut value = 0;
        while value < 256 {
            tables[byte][value] = advance((value as u32) << (byte * 8));
            value += 1;
        }
        byte += 1;
    }
    tables
}

const JUMP: [[u32; 256]; 4] = tables();

#[expect(clippy::indexing_slicing, reason = "Byte values always index within 256-entry tables")]
pub(super) fn jump(state: u32) -> u32 {
    let bytes = state.to_le_bytes();
    // Each lookup is bounded by a byte. Xorshift is linear over XOR, so the
    // transformed byte contributions reconstruct the transformed whole state.
    JUMP[0][usize::from(bytes[0])]
        ^ JUMP[1][usize::from(bytes[1])]
        ^ JUMP[2][usize::from(bytes[2])]
        ^ JUMP[3][usize::from(bytes[3])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_matches_63_serial_steps() {
        for bit in 0..32 {
            assert_eq!(jump(1 << bit), advance(1 << bit));
        }
        for state in [0, 1, 42, 0x6d2b_79f5, u32::MAX] {
            assert_eq!(jump(state), advance(state));
        }
    }
}
