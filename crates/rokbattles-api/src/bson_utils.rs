use mongodb::bson::{Bson, Document};

/// Follow a nested BSON path and return the final document.
pub(crate) fn nested_document<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Document> {
    let mut current = document;

    for key in path {
        current = current.get_document(*key).ok()?;
    }

    Some(current)
}

/// Read a nested value and convert known BSON number types to `i64`.
pub(crate) fn nested_i64(document: &Document, path: &[&str]) -> Option<i64> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(bson_to_i64)
}

/// Read a nested value and convert known BSON number types to an exact `i64`.
///
/// Floating point values are only accepted when they are finite whole numbers.
pub(crate) fn nested_i64_exact(document: &Document, path: &[&str]) -> Option<i64> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(bson_to_i64_exact)
}

/// Read a nested value and convert known BSON number types to `f64`.
pub(crate) fn nested_f64(document: &Document, path: &[&str]) -> Option<f64> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(bson_to_f64)
}

/// Read a nested value as a boolean.
pub(crate) fn nested_bool(document: &Document, path: &[&str]) -> Option<bool> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(|value| match value {
        Bson::Boolean(value) => Some(*value),
        _ => None,
    })
}

/// Read a nested value as a string slice.
pub(crate) fn nested_str<'a>(document: &'a Document, path: &[&str]) -> Option<&'a str> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get_str(path[path.len() - 1]).ok()
}

/// Read a nested value as an owned string.
pub(crate) fn nested_string(document: &Document, path: &[&str]) -> Option<String> {
    nested_str(document, path).map(ToOwned::to_owned)
}

/// Read a nested value as an array.
pub(crate) fn nested_array<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Vec<Bson>> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(mongodb::bson::Bson::as_array)
}

/// Convert known BSON number types to `i64`.
///
/// Floating point values are truncated toward zero.
pub(crate) fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) => {
            if value.is_finite() {
                Some(*value as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Convert known BSON number types to an exact `i64`.
///
/// Floating point values are only accepted when they are finite whole numbers.
pub(crate) fn bson_to_i64_exact(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) => {
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= i64::MIN as f64
                && *value <= i64::MAX as f64
            {
                Some(*value as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Convert BSON values to `i64`, accepting numeric strings as well.
pub(crate) fn bson_to_i64_loose(value: &Bson) -> Option<i64> {
    bson_to_i64_exact(value).or_else(|| match value {
        Bson::String(raw) => raw.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// Convert known BSON number types to `f64`.
pub(crate) fn bson_to_f64(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(value) => Some(f64::from(*value)),
        Bson::Int64(value) => Some(*value as f64),
        Bson::Double(value) => {
            if value.is_finite() {
                Some(*value)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Convert BSON values to `f64`, accepting numeric strings as well.
pub(crate) fn bson_to_f64_loose(value: &Bson) -> Option<f64> {
    bson_to_f64(value).or_else(|| match value {
        Bson::String(raw) => raw.trim().parse::<f64>().ok().filter(|parsed| parsed.is_finite()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::*;

    #[test]
    fn reads_nested_number_types() {
        let document = doc! {
            "outer": {
                "inner": {
                    "int": 10_i64,
                    "float": 20.9,
                }
            }
        };

        assert_eq!(nested_i64(&document, &["outer", "inner", "int"]), Some(10));
        assert_eq!(nested_i64(&document, &["outer", "inner", "float"]), Some(20));
        assert_eq!(nested_i64_exact(&document, &["outer", "inner", "float"]), None);
    }

    #[test]
    fn converts_numeric_bson_variants() {
        assert_eq!(bson_to_i64(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_to_i64(&Bson::Double(56.8)), Some(56));
        assert_eq!(bson_to_i64(&Bson::Double(f64::INFINITY)), None);

        assert_eq!(bson_to_i64_exact(&Bson::Double(56.0)), Some(56));
        assert_eq!(bson_to_i64_exact(&Bson::Double(56.8)), None);

        assert_eq!(bson_to_f64(&Bson::Int32(12)), Some(12.0));
        assert_eq!(bson_to_f64(&Bson::Double(56.8)), Some(56.8));
        assert_eq!(bson_to_f64(&Bson::Double(f64::NAN)), None);

        assert_eq!(bson_to_i64_loose(&Bson::String(" 56 ".to_string())), Some(56));
        assert_eq!(bson_to_f64_loose(&Bson::String(" 56.8 ".to_string())), Some(56.8));
        assert_eq!(bson_to_f64_loose(&Bson::String("nan".to_string())), None);
    }
}
