//! Helpers for numeric-keyed containers.

use serde_json::{Map, Value};

/// Convert alternating numeric key/value containers into the JSON shape.
pub(crate) fn numeric_keyed_container(items: &[Value]) -> Option<Value> {
    if items.is_empty() || !items.len().is_multiple_of(2) {
        return None;
    }

    let mut pairs = Vec::with_capacity(items.len() / 2);
    for (index, pair) in items.chunks_exact(2).enumerate() {
        let key = integer_key(&pair[0])?;
        pairs.push((key, index + 1, pair[1].clone()));
    }

    if pairs.iter().all(|(key, expected, _value)| key == expected) {
        return Some(Value::Array(
            pairs.into_iter().map(|(_key, _expected, value)| value).collect(),
        ));
    }

    let mut map = Map::new();
    for (key, _expected, value) in pairs {
        // Non-sequential numeric keys are semantic ids, such as troop ids or
        // buff ids. JSON object keys are strings, so keep them that way.
        let previous = map.insert(key.to_string(), value);
        if previous.is_some() {
            return None;
        }
    }

    Some(Value::Object(map))
}

fn integer_key(value: &Value) -> Option<usize> {
    let number = value.as_u64()?;
    usize::try_from(number).ok()
}
