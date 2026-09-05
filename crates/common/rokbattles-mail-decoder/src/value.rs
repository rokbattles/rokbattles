//! Converts decoded table items into JSON arrays or objects.
//!
//! The wire format supplies a sequence of values without identifying keys.
//! Classification therefore depends on the whole sequence: item count first,
//! then the types of the values in potential key positions.

use std::collections::BTreeMap;

use serde_json::{Map, Value, map::Entry};

use crate::common::DecodeError;

#[derive(Debug)]
pub(crate) struct ClassifiedTable {
    pub(crate) value: Value,
}

pub(crate) fn classify_table(
    items: Vec<Value>,
    table_offset: usize,
) -> Result<ClassifiedTable, DecodeError> {
    if items.is_empty() || !items.len().is_multiple_of(2) {
        return Ok(sequential(items));
    }

    let mut has_string_keys = false;
    let mut has_number_keys = false;
    for pair in items.as_chunks::<2>().0 {
        match &pair[0] {
            Value::String(_) => has_string_keys = true,
            Value::Number(_) => has_number_keys = true,
            // A non-key value makes the entire table a sequence, even if earlier
            // positions looked like keys of conflicting types.
            _ => return Ok(sequential(items)),
        }
    }

    // Converting both key types to strings could merge distinct keys, such as 1 and "1".
    if has_string_keys && has_number_keys {
        return Err(DecodeError::MixedTableKeyTypes { offset: table_offset });
    }

    let value = if has_string_keys {
        string_keyed_table(items, table_offset)?
    } else {
        numeric_keyed_table(items, table_offset)?
    };
    Ok(ClassifiedTable { value })
}

fn sequential(items: Vec<Value>) -> ClassifiedTable {
    ClassifiedTable { value: Value::Array(items) }
}

fn string_keyed_table(items: Vec<Value>, table_offset: usize) -> Result<Value, DecodeError> {
    let mut map = Map::with_capacity(items.len() / 2);
    for (key, value) in owned_pairs(items) {
        let Value::String(key) = key else {
            unreachable!("table key types were classified before conversion");
        };
        match map.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
            Entry::Occupied(entry) => {
                return Err(DecodeError::DuplicateTableKey {
                    offset: table_offset,
                    key: entry.key().clone(),
                });
            }
        }
    }
    Ok(Value::Object(map))
}

fn numeric_keyed_table(items: Vec<Value>, table_offset: usize) -> Result<Value, DecodeError> {
    let pair_count = items.len() / 2;
    // N distinct integer keys within 1..=N cover every array position. Their order
    // in the file does not matter; duplicates must fall through to object validation.
    let is_sequential = {
        let mut keys = vec![false; pair_count];
        items.as_chunks::<2>().0.iter().all(|pair| {
            let Some(key) = pair[0].as_u64().and_then(|key| usize::try_from(key).ok()) else {
                return false;
            };
            key > 0 && key <= pair_count && !std::mem::replace(&mut keys[key - 1], true)
        })
    };

    if is_sequential {
        // Numeric ordering maps the file's one-based keys to zero-based array positions.
        let mut values = BTreeMap::new();
        for (key, value) in owned_pairs(items) {
            let Some(key) = key.as_u64().and_then(|key| usize::try_from(key).ok()) else {
                unreachable!("sequential numeric keys were validated before conversion");
            };
            values.insert(key, value);
        }
        return Ok(Value::Array(values.into_values().collect()));
    }

    let mut map = Map::with_capacity(pair_count);
    for (key, value) in owned_pairs(items) {
        let Value::Number(key) = key else {
            unreachable!("table key types were classified before conversion");
        };
        // Detect duplicates after rendering, since these strings become the JSON keys.
        let key = key.to_string();
        match map.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
            Entry::Occupied(entry) => {
                return Err(DecodeError::DuplicateTableKey {
                    offset: table_offset,
                    key: entry.key().clone(),
                });
            }
        }
    }
    Ok(Value::Object(map))
}

// Classification guarantees an even item count. Moving each pair avoids cloning
// decoded strings and nested containers during conversion.
fn owned_pairs(items: Vec<Value>) -> impl Iterator<Item = (Value, Value)> {
    let mut items = items.into_iter();
    std::iter::from_fn(move || {
        let key = items.next()?;
        let Some(value) = items.next() else {
            unreachable!("pair iterator requires an even item count");
        };
        Some((key, value))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn sequential_numeric_keys_are_sorted_into_an_array() {
        let values = json!([2, "second", 1, "first"]);
        let classified =
            classify_table(values.as_array().expect("array").clone(), 0).expect("classify table");

        assert_eq!(classified.value, json!(["first", "second"]));
    }

    #[test]
    fn duplicate_numeric_keys_are_rejected() {
        let values = json!([2, "first", 2, "second"]);
        let error = classify_table(values.as_array().expect("array").clone(), 7)
            .expect_err("duplicate should fail");

        assert_eq!(error, DecodeError::DuplicateTableKey { offset: 7, key: "2".to_string() });
    }

    #[test]
    fn mixed_key_types_are_rejected() {
        let values = json!([1, "first", "key", "second"]);
        let error = classify_table(values.as_array().expect("array").clone(), 11)
            .expect_err("mixed keys should fail");

        assert_eq!(error, DecodeError::MixedTableKeyTypes { offset: 11 });
    }

    #[test]
    fn non_key_values_remain_a_sequence() {
        let values = json!([true, "first", false, "second"]);
        let classified =
            classify_table(values.as_array().expect("array").clone(), 0).expect("classify table");

        assert_eq!(classified.value, values);
    }
}
