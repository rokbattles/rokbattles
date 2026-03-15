//! Shared helpers for navigating AllianceAOOIndividualResults mail content.

use mail_processor_sdk::ExtractError;
use serde_json::{Map, Value};

/// Require an object field from a JSON map.
pub(crate) fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType { field, expected: "object" })
}

/// Require an unsigned integer field from a JSON map.
pub(crate) fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
}

/// Read an optional unsigned integer field from a JSON map.
pub(crate) fn optional_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(Value::Null),
        Some(value) => value
            .as_u64()
            .map(Value::from)
            .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" }),
    }
}

/// Require a boolean field from a JSON map.
pub(crate) fn require_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<bool, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_bool().ok_or(ExtractError::InvalidFieldType { field, expected: "boolean" })
}

/// Require a string field from a JSON map.
pub(crate) fn require_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<String, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "string" })
}

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
            .ok_or(ExtractError::InvalidFieldType { field, expected: "object" }),
    }
}

/// Read an optional object field, treating an empty array as missing.
///
/// Some individual-results payloads encode absent stats as `[]` instead of `null`.
pub(crate) fn optional_child_object_or_empty_array<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<Option<&'a Map<String, Value>>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(values)) if values.is_empty() => Ok(None),
        Some(value) => value
            .as_object()
            .map(Some)
            .ok_or(ExtractError::InvalidFieldType { field, expected: "object" }),
    }
}
