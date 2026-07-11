//! Helpers for walking AllianceAOOIndividualResults mail content.

pub(crate) use mail_processor_sdk::{
    ExtractError, optional_child_object, optional_child_object_or_empty_array,
    optional_u64_field as sdk_optional_u64_field,
    optional_u64_field_or_zero as sdk_optional_u64_field_or_zero, require_bool_field,
    require_child_object, require_string_field, require_u64_field,
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
    Ok(Value::from(sdk_optional_u64_field_or_zero(object, field)?))
}
