#![forbid(unsafe_code)]

//! Processes carriage reports containing resources sent or received by a governor.

mod carriage;
mod metadata;

use rokbattles_mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Extracts the `metadata` and `carriage` sections of a decoded carriage report.
///
/// The target identifies the other governor in the transfer. The `active` flag
/// is true for sent resources and false for received resources. Resource entries
/// are returned in report order, and tax is returned as a fraction.
///
/// Missing or null direction and tax fields default to false and zero.
///
/// # Errors
///
/// Returns [`ProcessError::ExtractorFailed`] if a required field is missing or a
/// field has the wrong type.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor),
        Box::new(carriage::CarriageExtractor),
    ])
    .process(input)
}
