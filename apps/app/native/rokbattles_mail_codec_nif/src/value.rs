//! Converts JSON-compatible BEAM values without unbounded native recursion.

use rustler::{Term, TermType, types::map::MapIterator};
use serde_json::{Map, Number, Value};

// Match the shared codec's table limit before building its input tree.
const MAX_DEPTH: usize = 128;

pub(crate) fn from_term(term: Term<'_>, depth: usize) -> Result<Value, String> {
    match term.get_type() {
        TermType::Atom => term
            .decode::<Option<bool>>()
            .map(|value| value.map_or(Value::Null, Value::Bool))
            .map_err(|_| "expected a boolean or nil, not an atom".to_owned()),
        TermType::Integer => term
            .decode::<i64>()
            .map(Number::from)
            .or_else(|_| term.decode::<u64>().map(Number::from))
            .map(Value::Number)
            .map_err(|_| "integer exceeds the signed or unsigned 64-bit range".to_owned()),
        TermType::Float => term
            .decode::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| "expected a finite number".to_owned()),
        TermType::Binary => term
            .decode::<String>()
            .map(Value::String)
            .map_err(|_| "expected a UTF-8 string".to_owned()),
        TermType::List => {
            check_depth(depth)?;
            let mut values = Vec::new();
            let mut tail = term;
            while !tail.is_empty_list() {
                let (head, rest) =
                    tail.list_get_cell().map_err(|_| "expected a proper list".to_owned())?;
                values.push(from_term(head, depth + 1)?);
                tail = rest;
            }
            Ok(Value::Array(values))
        }
        TermType::Map => {
            check_depth(depth)?;
            let entries = MapIterator::new(term).ok_or_else(|| "expected a map".to_owned())?;
            let mut values = Map::new();
            for (key, value) in entries {
                let key = key
                    .decode::<String>()
                    .map_err(|_| "map keys must be UTF-8 strings".to_owned())?;
                values.insert(key, from_term(value, depth + 1)?);
            }
            Ok(Value::Object(values))
        }
        _ => Err("expected a JSON-compatible value".to_owned()),
    }
}

fn check_depth(depth: usize) -> Result<(), String> {
    if depth >= MAX_DEPTH {
        Err(format!("table nesting exceeds max depth of {MAX_DEPTH}"))
    } else {
        Ok(())
    }
}
