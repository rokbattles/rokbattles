//! Helpers for normalizing numeric-keyed containers.

use serde_json::{Map, Value};

pub(crate) fn numeric_keyed_container(items: &[Value]) -> Option<Value> {
    if items.is_empty() || !items.len().is_multiple_of(2) {
        return None;
    }

    let mut pairs = Vec::with_capacity(items.len() / 2);
    for (index, pair) in items.as_chunks::<2>().0.iter().enumerate() {
        let key = integer_key(&pair[0])?;
        pairs.push((key, index + 1, pair[1].clone()));
    }

    if pairs.iter().all(|(key, expected, _)| key == expected) {
        return Some(Value::Array(pairs.into_iter().map(|(_, _, value)| value).collect()));
    }

    let mut map = Map::new();
    for (key, _, value) in pairs {
        // Non-sequential numeric keys are semantic ids. JSON object keys are
        // strings, so preserve those ids using their decimal representation.
        if map.insert(key.to_string(), value).is_some() {
            return None;
        }
    }

    Some(Value::Object(map))
}

fn integer_key(value: &Value) -> Option<usize> {
    usize::try_from(value.as_u64()?).ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn duplicate_numeric_keys_remain_an_array() {
        let values = json!([2, "first", 2, "second"]);
        let values = values.as_array().expect("array");

        assert_eq!(numeric_keyed_container(values), None);
    }

    #[test]
    fn mixed_values_remain_an_array() {
        let values = json!([1, "first", "key", "second"]);
        let values = values.as_array().expect("array");

        assert_eq!(numeric_keyed_container(values), None);
    }
}
