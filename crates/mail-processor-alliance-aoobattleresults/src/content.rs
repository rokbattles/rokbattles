//! Shared helpers for navigating AllianceAOOBattleResults mail content.

use mail_processor_sdk::{ExtractError, require_object};
use serde_json::{Map, Value};

/// Require the `body.kvs` object from the root mail payload.
pub(crate) fn require_body_kvs(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "kvs")
}

/// Require an object field from a JSON map.
pub(crate) fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType { field, expected: "object" })
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

/// Require an unsigned integer field from a JSON map.
pub(crate) fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
}

/// Require a boolean field from a JSON map.
pub(crate) fn require_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<bool, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_bool().ok_or(ExtractError::InvalidFieldType { field, expected: "boolean" })
}

/// Require a numeric field from a JSON map, preserving its numeric representation.
pub(crate) fn require_number_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    if value.is_number() {
        Ok(value.clone())
    } else {
        Err(ExtractError::InvalidFieldType { field, expected: "number" })
    }
}
