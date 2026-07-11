#![forbid(unsafe_code)]

//! Shared pieces for the mail processors.
//!
//! This crate holds the extractor traits, processor runtime, and JSON helpers
//! the mail-processor crates reuse.

mod error;
mod extract;
mod processor;
mod types;

pub use error::{ExtractError, ProcessError};
pub use extract::{
    BaseMetadata, extract_base_metadata, indexed_array_values, optional_bool_field,
    optional_child_object, optional_child_object_or_empty_array, optional_i64_field,
    optional_number_field_or_zero, optional_string_field, optional_u64_field,
    optional_u64_field_or_zero, optional_u64_or_string_field, require_bool_field,
    require_child_object, require_i64_field, require_number_field, require_object, require_string,
    require_string_field, require_u64, require_u64_field, require_u64_or_string_field,
};
pub use processor::{Extractor, Processor};
pub use types::{ProcessedMail, Section};
