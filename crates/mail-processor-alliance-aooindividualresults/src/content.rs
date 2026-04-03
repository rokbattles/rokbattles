//! Helpers for walking AllianceAOOIndividualResults mail content.

pub(crate) use mail_processor_sdk::{
    ExtractError, optional_child_object, optional_u64_field as sdk_optional_u64_field,
    require_bool_field, require_child_object, require_string_field, require_u64_field,
};
use serde_json::{Map, Value};

/// Reads an optional unsigned integer field from a JSON map.
pub(crate) fn optional_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    Ok(sdk_optional_u64_field(object, field)?.map_or(Value::Null, Value::from))
}

/// Reads an optional unsigned integer field and defaults to zero when it is missing.
pub(crate) fn optional_u64_field_or_zero(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(Value::from(0_u64)),
        Some(value) => value
            .as_u64()
            .map(Value::from)
            .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" }),
    }
}

/// Reads an optional object field and treats an empty array as missing.
///
/// Some individual-results payloads use `[]` instead of `null` for missing stats.
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
