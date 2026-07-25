//! Shared constants and error types for encoding.

use thiserror::Error;

pub(crate) const TAG_BOOL: u8 = 0x01;
pub(crate) const TAG_F64: u8 = 0x03;
pub(crate) const TAG_STRING: u8 = 0x04;
pub(crate) const TAG_TABLE: u8 = 0x05;
pub(crate) const TABLE_END: u8 = 0xff;

pub(crate) const FILE_MARKER: u8 = 0xff;
pub(crate) const FILE_HEADER_LEN: usize = 9;
pub(crate) const CHECKSUM_SEED: u64 = 0x1505;

pub(crate) const MAX_DEPTH: usize = 128;

/// Errors returned by [`crate::encode`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodeError {
    /// A null value appeared where it could not be omitted.
    #[error("null cannot be represented by the persistent mail format")]
    NullValue,
    /// A number could not be represented as a finite `f64`.
    #[error("number cannot be represented as a finite f64")]
    UnrepresentableNumber,
    /// A string exceeded the format's `u32` byte-length field.
    #[error("string exceeds the persistent mail limit")]
    StringTooLong,
    /// An array was too large to assign a representable numeric key.
    #[error("array exceeds the persistent mail limit")]
    ArrayTooLong,
    /// A table exceeded the encoder's defensive nesting limit.
    #[error("table nesting exceeds max depth of {limit}")]
    DepthLimitExceeded {
        /// Maximum supported table nesting depth.
        limit: usize,
    },
    /// The fixed header was not present after initialization.
    #[error("persistent mail file header was not initialized")]
    HeaderNotInitialized,
}
