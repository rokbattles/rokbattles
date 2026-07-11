//! Shared processor limits.

/// Largest zlib-inflated payload the TCP processor will inspect.
pub(crate) const MAX_ZLIB_INFLATED_BYTES: usize = 1024 * 1024;
