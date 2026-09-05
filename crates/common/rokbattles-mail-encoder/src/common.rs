//! Wire-format constants and encoding errors.

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

/// An error encountered while encoding a `Persistent.Mail` file.
///
/// Returned by [`crate::encode`]. Encoding stops at the first error and discards
/// the partially written file.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodeError {
    /// The root value or an array element is null.
    ///
    /// Null object fields are omitted before their values are encoded.
    #[error("null cannot be represented by the persistent mail format")]
    NullValue,
    /// A JSON number cannot be converted to a finite `f64`.
    ///
    /// Loss of integer precision alone does not produce this error.
    #[error("number cannot be represented as a finite f64")]
    UnrepresentableNumber,
    /// A string value or object key exceeds `u32::MAX` UTF-8 bytes.
    #[error("string exceeds the persistent mail limit")]
    StringTooLong,
    /// An array element's one-based index cannot be converted to `u64`.
    ///
    /// This checks the integer index before it is written as `f64`; it does
    /// not check whether that floating-point conversion preserves precision.
    #[error("array exceeds the persistent mail limit")]
    ArrayTooLong,
    /// An object or array would exceed the table nesting limit.
    #[error("table nesting exceeds max depth of {limit}")]
    DepthLimitExceeded {
        /// The maximum number of nested tables (128), including the outermost table.
        limit: usize,
    },
    /// The output buffer has no complete checksum field to fill in.
    ///
    /// This indicates an internal invariant was violated: [`crate::encode`]
    /// reserves the header before appending any values.
    #[error("persistent mail file header was not initialized")]
    HeaderNotInitialized,
}
