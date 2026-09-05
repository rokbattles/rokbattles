#![forbid(unsafe_code)]

//! Extracts named sections from decoded mail JSON.
//!
//! Implement [`Extractor`] for each independent section and register the
//! extractors with a [`Processor`]. Each extractor borrows the same input and
//! returns an owned [`Section`]. The processor runs them on scoped threads and
//! collects their output into a [`ProcessedMail`]. Mail type detection and binary
//! decoding belong to the caller.
//!
//! # Extraction helpers
//!
//! Helpers check JSON types without modifying the input. Required fields report
//! missing keys separately from invalid values, including null. Optional fields
//! treat both missing keys and null as absent; a present value of the wrong type
//! still returns an error. Helpers ending in `_or_zero` replace absence with zero.
//!
//! Integer helpers require integer JSON numbers in the target type's range.
//! Only the `_u64_or_string_field` helpers also parse decimal strings. They do
//! not round floating-point numbers or trim strings. Number helpers preserve
//! the JSON number representation, and string helpers return owned copies.
//!
//! [`optional_child_object_or_empty_array`] also treats `[]` as absent because
//! the mail decoder represents empty tables as arrays. Other object helpers
//! reject arrays, including empty ones.
//!
//! # Output
//!
//! A section holds either named fields or an array of JSON values. Serialization
//! writes that object or array directly. Processed mail serializes as an object
//! keyed by section name, with no additional wrapper. Section names and object
//! field names are stored in sorted order; array element order is preserved.
//!
//! # Examples
//!
//! An extractor for the common metadata fields:
//!
//! ```
//! use rokbattles_mail_sdk::{
//!     ExtractError, Extractor, Processor, Section, extract_base_metadata,
//! };
//! use serde_json::{Value, json};
//!
//! struct Metadata;
//!
//! impl Extractor for Metadata {
//!     fn section(&self) -> &'static str {
//!         "metadata"
//!     }
//!
//!     fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
//!         Ok(extract_base_metadata(input)?.into_section())
//!     }
//! }
//!
//! let processor = Processor::new(vec![Box::new(Metadata)]);
//! let mail = json!({ "id": "mail-1", "time": 1234, "receiver": "player-1", "serverId": 55 });
//! let processed = processor.process(&mail)?;
//! assert_eq!(processed.sections()["metadata"].fields()["mail_id"], json!("mail-1"));
//! # Ok::<(), rokbattles_mail_sdk::ProcessError>(())
//! ```

mod error;
mod extract;
mod processor;
mod types;

pub use error::{ExtractError, ProcessError};
pub use extract::{
    BaseMetadata, extract_base_metadata, optional_bool_field, optional_child_object,
    optional_child_object_or_empty_array, optional_i64_field, optional_number_field_or_zero,
    optional_string_field, optional_u64_field, optional_u64_field_or_zero,
    optional_u64_or_string_field, require_array, require_bool_field, require_child_object,
    require_i64_field, require_number_field, require_object, require_string, require_string_field,
    require_u64, require_u64_field, require_u64_or_string_field,
};
pub use processor::{Extractor, Processor};
pub use types::{ProcessedMail, Section};
