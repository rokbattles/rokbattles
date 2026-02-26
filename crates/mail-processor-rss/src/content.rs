//! Shared helpers for navigating Rss mail content.

use mail_processor_sdk::{ExtractError, require_object};
use serde_json::{Map, Value};

/// Require the nested `body.content` object from an Rss mail payload.
pub(crate) fn require_content(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "content")
}

/// Require an object field from a JSON map.
pub(crate) fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "object",
    })
}

/// Require a numeric field from a JSON map, preserving its numeric representation.
pub(crate) fn require_number_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    if value.is_number() {
        Ok(value.clone())
    } else {
        Err(ExtractError::InvalidFieldType {
            field,
            expected: "number",
        })
    }
}

/// Require a numeric field, accepting either canonical or legacy source key.
pub(crate) fn require_number_field_alias(
    object: &Map<String, Value>,
    canonical: &'static str,
    alias: &'static str,
) -> Result<Value, ExtractError> {
    match object.get(canonical).or_else(|| object.get(alias)) {
        Some(value) if value.is_number() => Ok(value.clone()),
        Some(_) => Err(ExtractError::InvalidFieldType {
            field: canonical,
            expected: "number",
        }),
        None => Err(ExtractError::MissingField { field: canonical }),
    }
}
