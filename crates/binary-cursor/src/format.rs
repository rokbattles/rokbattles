//! Small constants for the binary format.

pub(crate) const TAG_BOOL: u8 = 0x01;
pub(crate) const TAG_F32: u8 = 0x02;
pub(crate) const TAG_F64: u8 = 0x03;
pub(crate) const TAG_STRING: u8 = 0x04;
pub(crate) const TAG_CONTAINER: u8 = 0x05;

pub(crate) const MAX_DEPTH: usize = 128;

/// Largest header prefix scanned before the payload.
pub(crate) const MAX_PREAMBLE_SCAN_BYTES: usize = 16;

pub(crate) fn is_known_tag(tag: u8) -> bool {
    matches!(tag, TAG_BOOL | TAG_F32 | TAG_F64 | TAG_STRING | TAG_CONTAINER)
}
