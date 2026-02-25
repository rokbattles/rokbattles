//! Shared helpers for navigating AllianceAOOIndividualResults mail content.

use mail_processor_sdk::ExtractError;
use serde_json::{Map, Value};

/// Read an optional object field from a JSON map.
pub(crate) fn optional_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<Option<&'a Map<String, Value>>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_object()
            .map(Some)
            .ok_or(ExtractError::InvalidFieldType {
                field,
                expected: "object",
            }),
    }
}
