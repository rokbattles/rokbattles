//! Typed reads from decoded JSON objects.
//!
//! Required reads distinguish absent keys from invalid values. Optional reads
//! treat missing keys and null as absent. Borrowed maps and slices reference
//! the original input; returned strings and JSON numbers are owned.

use serde_json::{Map, Value};

use crate::{ExtractError, Section};

/// Common mail metadata copied from the root object.
///
/// [`extract_base_metadata`] reads `id`, `time`, `receiver`, and `serverId`.
/// Values are renamed for output without converting timestamp units or IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseMetadata {
    /// The root `id` string.
    pub mail_id: String,
    /// The root `time` integer, with its original units unchanged.
    pub mail_time: u64,
    /// The root `receiver` string.
    pub mail_receiver: String,
    /// The root `serverId` integer.
    pub server_id: u64,
}

impl BaseMetadata {
    /// Consumes the metadata and returns an object section with its output fields.
    ///
    /// The keys are `mail_id`, `mail_time`, `mail_receiver`, and `server_id`.
    /// The caller or extractor assigns the section name, usually `metadata`.
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

/// Borrows the object map inside `value`.
///
/// # Errors
///
/// Returns [`ExtractError::NotObject`] if `value` is not an object.
pub fn require_object(value: &Value) -> Result<&Map<String, Value>, ExtractError> {
    value.as_object().ok_or(ExtractError::NotObject)
}

/// Borrows the object stored at `field`.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] if the key is absent, or
/// [`ExtractError::InvalidFieldType`] if its value is not an object, including null.
pub fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType { field, expected: "object" })
}

/// Copies a required string from an object root.
///
/// # Errors
///
/// Returns [`ExtractError::NotObject`] for a non-object root. Otherwise,
/// returns the errors described by [`require_string_field`].
pub fn require_string(input: &Value, field: &'static str) -> Result<String, ExtractError> {
    let object = require_object(input)?;
    require_string_field(object, field)
}

/// Reads a required `u64` from an object root.
///
/// # Errors
///
/// Returns [`ExtractError::NotObject`] for a non-object root. Otherwise,
/// returns the errors described by [`require_u64_field`].
pub fn require_u64(input: &Value, field: &'static str) -> Result<u64, ExtractError> {
    let object = require_object(input)?;
    require_u64_field(object, field)
}

/// Copies the string at `field`.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] for an absent key, or
/// [`ExtractError::InvalidFieldType`] for a non-string value, including null.
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

/// Reads an integer JSON number representable as `u64`.
///
/// Strings and floating-point numbers are rejected, even if they represent
/// a whole number. Use [`require_u64_or_string_field`] to accept numeric strings.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] for an absent key, or
/// [`ExtractError::InvalidFieldType`] if its value cannot be read as `u64`.
pub fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
}

/// Reads an integer JSON number representable as `i64`.
///
/// Accepts signed and unsigned integer representations within `i64` bounds.
/// Strings and floating-point numbers are rejected.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] for an absent key, or
/// [`ExtractError::InvalidFieldType`] for the wrong type or an out-of-range integer.
pub fn require_i64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<i64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value_to_i64(value)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "signed 64-bit integer" })
}

/// Reads a `u64` from an integer JSON number or a decimal string.
///
/// Strings use Rust's `u64` parser without trimming whitespace. Floating-point
/// JSON numbers and fractional strings such as `"8.5"` are rejected.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] for an absent key, or
/// [`ExtractError::InvalidFieldType`] if conversion or string parsing fails.
pub fn require_u64_or_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value_to_u64(value)
        .ok_or(ExtractError::InvalidFieldType { field, expected: "unsigned integer" })
}

/// Reads the boolean at `field`.
///
/// Numeric and string representations of booleans are not converted.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] for an absent key, or
/// [`ExtractError::InvalidFieldType`] for a non-boolean value, including null.
pub fn require_bool_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<bool, ExtractError> {
    let value = object.get(field).ok_or(ExtractError::MissingField { field })?;
    value.as_bool().ok_or(ExtractError::InvalidFieldType { field, expected: "boolean" })
}

/// Clones the JSON number at `field`, preserving its representation.
///
/// The result remains a [`Value::Number`]; no integer or floating-point cast
/// is performed.
///
/// # Errors
///
/// Returns [`ExtractError::MissingField`] for an absent key, or
/// [`ExtractError::InvalidFieldType`] for a non-number value, including null.
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

/// Copies the string at `field`, returning `None` for a missing key or null.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] for a present, non-null
/// value that is not a string.
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

/// Reads an optional integer JSON number representable as `u64`.
///
/// A missing key or null returns `None`. Strings and floating-point numbers
/// are rejected, as in [`require_u64_field`].
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] if a non-null value cannot
/// be read as `u64`.
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

/// Reads an optional integer JSON number representable as `i64`.
///
/// A missing key or null returns `None`. Conversion follows
/// [`require_i64_field`], including its range checks.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] for a present, non-null
/// value with the wrong type or an out-of-range integer.
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

/// Reads an optional `u64` from an integer JSON number or decimal string.
///
/// A missing key or null returns `None`. Conversion follows
/// [`require_u64_or_string_field`].
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] if a present, non-null
/// value cannot be converted or parsed as `u64`.
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

/// Reads an optional `u64`, returning zero for a missing key or null.
///
/// Uses [`optional_u64_field`]; invalid values are not replaced with zero.
///
/// # Errors
///
/// Returns the errors from [`optional_u64_field`].
pub fn optional_u64_field_or_zero(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    Ok(optional_u64_field(object, field)?.unwrap_or_default())
}

/// Reads a boolean, returning `None` for a missing key or null.
///
/// Numeric and string representations of booleans are not converted.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] for a present, non-null
/// value that is not a boolean.
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

/// Clones a JSON number, returning integer zero for a missing key or null.
///
/// Present numbers keep their JSON representation. Numeric strings are rejected.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] for a present, non-null
/// value that is not a number.
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

/// Borrows a child object, returning `None` for a missing key or null.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] for a present, non-null
/// value that is not an object. This includes empty arrays.
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

/// Borrows a child object, treating a missing key, null, or `[]` as absent.
///
/// Use this for optional mail tables: the decoder represents an empty table
/// as `[]`, even when the processor otherwise expects object fields. An empty
/// object still returns `Some`.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] for a nonempty array or
/// a non-null scalar value.
pub fn optional_child_object_or_empty_array<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<Option<&'a Map<String, Value>>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        // Empty mail tables carry no object/array distinction after decoding.
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

/// Copies the required `id`, `time`, `receiver`, and `serverId` root fields.
///
/// The ID and receiver must be strings; time and server ID must be integer
/// JSON numbers representable as `u64`. Numeric strings are not accepted.
/// Fields are checked in the order listed. See [`BaseMetadata::into_section`]
/// for the output names.
///
/// # Errors
///
/// Returns [`ExtractError::NotObject`] for a non-object root,
/// [`ExtractError::MissingField`] for the first absent key, or
/// [`ExtractError::InvalidFieldType`] for the first invalid value.
pub fn extract_base_metadata(input: &Value) -> Result<BaseMetadata, ExtractError> {
    Ok(BaseMetadata {
        mail_id: require_string(input, "id")?,
        mail_time: require_u64(input, "time")?,
        mail_receiver: require_string(input, "receiver")?,
        server_id: require_u64(input, "serverId")?,
    })
}

/// Borrows all elements of an array value, including an empty array.
///
/// `field` is used only as an error label; this function checks `value`
/// directly rather than looking up a key.
///
/// # Errors
///
/// Returns [`ExtractError::InvalidFieldType`] if `value` is not an array.
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
        require_i64_field(object, "positive").expect_err("out-of-range unsigned value");
    }

    #[test]
    fn numeric_string_fields_accept_numbers_and_strings() {
        let value = json!({ "number": 7, "string": "8", "invalid": "8.5" });
        let object = value.as_object().expect("object");

        assert_eq!(require_u64_or_string_field(object, "number"), Ok(7));
        assert_eq!(require_u64_or_string_field(object, "string"), Ok(8));
        require_u64_or_string_field(object, "invalid").expect_err("non-integer string");
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
