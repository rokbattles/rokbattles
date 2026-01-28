//! Helper functions for pulling typed values from decoded JSON.

use serde_json::{Map, Value};

use crate::ExtractError;

/// Require that a JSON value is an object and return its map.
pub fn require_object(value: &Value) -> Result<&Map<String, Value>, ExtractError> {
    value.as_object().ok_or(ExtractError::NotObject)
}

/// Require a string field on a decoded mail object.
pub fn require_string(input: &Value, field: &'static str) -> Result<String, ExtractError> {
    let object = require_object(input)?;
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or(ExtractError::InvalidFieldType {
            field,
            expected: "string",
        })
}

/// Require an unsigned integer field on a decoded mail object.
pub fn require_u64(input: &Value, field: &'static str) -> Result<u64, ExtractError> {
    let object = require_object(input)?;
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "unsigned integer",
    })
}

/// Read array values, skipping index markers if the array is index/value pairs.
pub fn indexed_array_values<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<Vec<&'a Value>, ExtractError> {
    let array = value.as_array().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "array",
    })?;

    if is_indexed_array(array) {
        Ok(array.iter().skip(1).step_by(2).collect())
    } else {
        Ok(array.iter().collect())
    }
}

fn is_indexed_array(array: &[Value]) -> bool {
    if array.len() < 2 || !array.len().is_multiple_of(2) {
        return false;
    }

    let mut expected = match array.first().and_then(Value::as_u64) {
        Some(value) if value == 0 || value == 1 => value,
        _ => return false,
    };

    for value in array.iter().step_by(2) {
        let index = match value.as_u64() {
            Some(index) => index,
            None => return false,
        };
        if index != expected {
            return false;
        }
        expected += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExtractError;
    use serde_json::{Value, json};

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
    fn indexed_array_values_skips_index_pairs() {
        let input = json!([1, "a", 2, "b"]);
        let values = indexed_array_values(&input, "values").unwrap();
        let values: Vec<Value> = values.into_iter().cloned().collect();
        assert_eq!(values, vec![json!("a"), json!("b")]);
    }

    #[test]
    fn indexed_array_values_supports_numeric_values() {
        let input = json!([1, 10001, 2, 2]);
        let values = indexed_array_values(&input, "values").unwrap();
        let values: Vec<Value> = values.into_iter().cloned().collect();
        assert_eq!(values, vec![json!(10001), json!(2)]);
    }

    #[test]
    fn indexed_array_values_keeps_plain_arrays() {
        let input = json!([1, 2, 3]);
        let values = indexed_array_values(&input, "values").unwrap();
        let values: Vec<Value> = values.into_iter().cloned().collect();
        assert_eq!(values, vec![json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn indexed_array_values_rejects_non_arrays() {
        let input = json!("nope");
        let err = indexed_array_values(&input, "values").unwrap_err();
        assert!(matches!(err, ExtractError::InvalidFieldType { .. }));
    }
}
