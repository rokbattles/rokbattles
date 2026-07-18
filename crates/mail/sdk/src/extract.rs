//! Shared JSON extraction helpers.

use serde_json::{Map, Value};

use crate::{ExtractError, Section};

/// Common top-level metadata pulled from a decoded mail object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseMetadata {
    /// Mail identifier.
    pub mail_id: String,
    /// Mail timestamp.
    pub mail_time: u64,
    /// Mail receiver.
    pub mail_receiver: String,
    /// Mail server.
    pub server_id: u64,
}

impl BaseMetadata {
    /// Turns the metadata into the standard `metadata` section.
    #[must_use]
    pub fn into_section(self) -> Section {
        let mut section = Section::new();
        section.insert("mail_id", Value::String(self.mail_id));
        section.insert("mail_time", Value::from(self.mail_time));
        section.insert("mail_receiver", Value::String(self.mail_receiver));
        section.insert("server_id", Value::from(self.server_id));
        section
    }
}

/// Returns the JSON object map or an extraction error.
pub fn require_object(value: &Value) -> Result<&Map<String, Value>, ExtractError> {
    value.as_object().ok_or(ExtractError::NotObject)
}

/// Reads a required object field from a JSON map.
pub fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType { field, expected: "object" })
}

/// Reads a required string field from a decoded mail object.
pub fn require_string(input: &Value, field: &'static str) -> Result<String, ExtractError> {
    let object = require_object(input)?;
    require_string_field(object, field)
}

/// Reads a required unsigned integer field from a decoded mail object.
pub fn require_u64(input: &Value, field: &'static str) -> Result<u64, ExtractError> {
    let object = require_object(input)?;
    require_u64_field(object, field)
}

/// Reads a required string field from a JSON map.
pub fn require_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<String, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "string" })
}

/// Reads a required unsigned integer field from a JSON map.
pub fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
}

/// Reads a required signed integer field from a JSON map.
pub fn require_i64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<i64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value_to_i64(value)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "signed 64-bit integer" })
}

/// Reads a required unsigned integer field that may be encoded as a numeric string.
pub fn require_u64_or_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value_to_u64(value)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
}

/// Reads a required boolean field from a JSON map.
pub fn require_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<bool, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_bool().ok_or(ExtractError::InvalidFieldType { field, expected: "boolean" })
}

/// Reads a required numeric field and keeps its JSON representation intact.
pub fn require_number_field(
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

/// Reads an optional string field from a JSON map.
pub fn optional_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<String>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        _ => Err(ExtractError::InvalidFieldType { field, expected: "string" }),
    }
}

/// Reads an optional unsigned integer field from a JSON map.
pub fn optional_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
            .map(Some),
    }
}

/// Reads an optional signed integer field from a JSON map.
pub fn optional_i64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<i64>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value_to_i64(value)
            .map(Some)
            .ok_or(ExtractError::InvalidFieldType { field, expected: "signed 64-bit integer" }),
    }
}

/// Reads an optional unsigned integer field that may be encoded as a numeric string.
pub fn optional_u64_or_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value_to_u64(value)
            .map(Some)
            .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" }),
    }
}

/// Reads an optional unsigned integer field and defaults to zero when absent.
pub fn optional_u64_field_or_zero(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    Ok(optional_u64_field(object, field)?.unwrap_or_default())
}

/// Reads an optional boolean field from a JSON map.
pub fn optional_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<bool>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .ok_or(ExtractError::InvalidFieldType { field, expected: "boolean" })
            .map(Some),
    }
}

/// Reads an optional number field and defaults to zero when it is missing.
pub fn optional_number_field_or_zero(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(Value::from(0)),
        Some(value) if value.is_number() => Ok(value.clone()),
        Some(_) => Err(ExtractError::InvalidFieldType { field, expected: "number" }),
    }
}

/// Reads an optional object field from a JSON map.
pub fn optional_child_object<'a>(
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

/// Reads an optional object field, treating an empty array as absent.
pub fn optional_child_object_or_empty_array<'a>(
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

fn value_to_i64(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn value_to_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

/// Reads the top-level metadata fields most mail types share.
pub fn extract_base_metadata(input: &Value) -> Result<BaseMetadata, ExtractError> {
    Ok(BaseMetadata {
        mail_id: require_string(input, "id")?,
        mail_time: require_u64(input, "time")?,
        mail_receiver: require_string(input, "receiver")?,
        server_id: require_u64(input, "serverId")?,
    })
}

/// Reads the values from a decoded JSON array.
pub fn require_array<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a [Value], ExtractError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "array" })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::ExtractError;

    #[test]
    fn require_string_reads_value() {
        let input = json!({ "name": "battle" });
        let value = require_string(&input, "name").unwrap();
        assert_eq!(value, "battle");
    }

    #[test]
    fn require_string_rejects_non_string() {
        let input = json!({ "name": 42 });
        let err = require_string(&input, "name").unwrap_err();
        assert!(matches!(err, ExtractError::InvalidFieldType { .. }));
    }

    #[test]
    fn require_u64_reads_value() {
        let input = json!({ "time": 1234 });
        let value = require_u64(&input, "time").unwrap();
        assert_eq!(value, 1234);
    }

    #[test]
    fn require_u64_rejects_non_number() {
        let input = json!({ "time": "soon" });
        let err = require_u64(&input, "time").unwrap_err();
        assert!(matches!(err, ExtractError::InvalidFieldType { .. }));
    }

    #[test]
    fn require_child_object_reads_nested_object() {
        let input = json!({ "body": { "content": true } });
        let object = require_object(&input).unwrap();
        let body = require_child_object(object, "body").unwrap();
        assert_eq!(body["content"], json!(true));
    }

    #[test]
    fn optional_u64_field_reads_number() {
        let input = json!({ "count": 7 });
        let object = require_object(&input).unwrap();
        assert_eq!(optional_u64_field(object, "count").unwrap(), Some(7));
    }

    #[test]
    fn signed_fields_accept_signed_and_in_range_unsigned_values() {
        let value = json!({ "negative": -7, "positive": u64::MAX });
        let object = value.as_object().expect("object");

        assert_eq!(require_i64_field(object, "negative"), Ok(-7));
        assert!(require_i64_field(object, "positive").is_err());
    }

    #[test]
    fn numeric_string_fields_accept_numbers_and_strings() {
        let value = json!({ "number": 7, "string": "8", "invalid": "8.5" });
        let object = value.as_object().expect("object");

        assert_eq!(require_u64_or_string_field(object, "number"), Ok(7));
        assert_eq!(require_u64_or_string_field(object, "string"), Ok(8));
        assert!(require_u64_or_string_field(object, "invalid").is_err());
    }

    #[test]
    fn optional_object_treats_empty_array_as_missing() {
        let value = json!({ "missing": [], "present": {} });
        let object = value.as_object().expect("object");

        assert_eq!(optional_child_object_or_empty_array(object, "missing"), Ok(None));
        assert!(optional_child_object_or_empty_array(object, "present").unwrap().is_some());
    }

    #[test]
    fn optional_number_field_or_zero_defaults_missing_values() {
        let input = json!({});
        let object = require_object(&input).unwrap();
        assert_eq!(optional_number_field_or_zero(object, "count").unwrap(), json!(0));
    }

    #[test]
    fn extract_base_metadata_reads_common_fields() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55
        });
        let metadata = extract_base_metadata(&input).unwrap();
        let section = metadata.into_section();
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("mail-1"));
        assert_eq!(fields["mail_time"], json!(1234));
        assert_eq!(fields["mail_receiver"], json!("player-1"));
        assert_eq!(fields["server_id"], json!(55));
    }

    #[test]
    fn require_array_returns_every_value() {
        let input = json!([1, 2, 3]);
        let values = require_array(&input, "values").unwrap();
        assert_eq!(values, [json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn require_array_rejects_non_arrays() {
        let input = json!("nope");
        let err = require_array(&input, "values").unwrap_err();
        assert!(matches!(err, ExtractError::InvalidFieldType { .. }));
    }
}
