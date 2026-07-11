#![forbid(unsafe_code)]

//! Shared BSON access and numeric conversion helpers.

use mongodb::bson::{Bson, Document};

/// Follow a nested BSON path and return its value.
#[must_use]
pub fn nested_value<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Bson> {
    let (key, parents) = path.split_last()?;
    let parent = nested_document(document, parents)?;
    parent.get(*key)
}

/// Follow a nested BSON path and return the final document.
#[must_use]
pub fn nested_document<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Document> {
    path.iter().try_fold(document, |current, key| current.get_document(*key).ok())
}

/// Read a nested value and truncate known numeric BSON types to `i64`.
#[must_use]
pub fn nested_i64(document: &Document, path: &[&str]) -> Option<i64> {
    nested_value(document, path).and_then(bson_to_i64)
}

/// Read a nested value and convert known numeric BSON types exactly to `i64`.
#[must_use]
pub fn nested_i64_exact(document: &Document, path: &[&str]) -> Option<i64> {
    nested_value(document, path).and_then(bson_to_i64_exact)
}

/// Read a nested value and convert known numeric BSON types to `f64`.
#[must_use]
pub fn nested_f64(document: &Document, path: &[&str]) -> Option<f64> {
    nested_value(document, path).and_then(bson_to_f64)
}

/// Read a nested value as a boolean.
#[must_use]
pub fn nested_bool(document: &Document, path: &[&str]) -> Option<bool> {
    nested_value(document, path).and_then(Bson::as_bool)
}

/// Read a nested value as a string slice.
#[must_use]
pub fn nested_str<'a>(document: &'a Document, path: &[&str]) -> Option<&'a str> {
    nested_value(document, path).and_then(Bson::as_str)
}

/// Read a nested value as an owned string.
#[must_use]
pub fn nested_string(document: &Document, path: &[&str]) -> Option<String> {
    nested_str(document, path).map(ToOwned::to_owned)
}

/// Read a nested value as an array.
#[must_use]
pub fn nested_array<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Vec<Bson>> {
    nested_value(document, path).and_then(Bson::as_array)
}

/// Convert BSON integer storage types to `i64`, rejecting doubles.
#[must_use]
pub fn bson_integer_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        _ => None,
    }
}

/// Convert known numeric BSON types to `i64`, truncating doubles toward zero.
#[must_use]
pub fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Double(value)
            if value.is_finite() && *value >= i64::MIN as f64 && *value < -(i64::MIN as f64) =>
        {
            Some(*value as i64)
        }
        _ => bson_integer_to_i64(value),
    }
}

/// Convert known numeric BSON types to `u64`, truncating doubles toward zero.
#[must_use]
pub fn bson_to_u64(value: &Bson) -> Option<u64> {
    u64::try_from(bson_to_i64(value)?).ok()
}

/// Convert known numeric BSON types exactly to `i64`.
#[must_use]
pub fn bson_to_i64_exact(value: &Bson) -> Option<i64> {
    match value {
        Bson::Double(value)
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= i64::MIN as f64
                && *value < -(i64::MIN as f64) =>
        {
            Some(*value as i64)
        }
        _ => bson_integer_to_i64(value),
    }
}

/// Convert known numeric BSON types exactly to `i32`.
#[must_use]
pub fn bson_to_i32_exact(value: &Bson) -> Option<i32> {
    i32::try_from(bson_to_i64_exact(value)?).ok()
}

/// Convert BSON values exactly to `i64`, accepting numeric strings.
#[must_use]
pub fn bson_to_i64_loose(value: &Bson) -> Option<i64> {
    bson_to_i64_exact(value).or_else(|| match value {
        Bson::String(raw) => raw.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// Convert known numeric BSON types to a finite `f64`.
#[must_use]
pub fn bson_to_f64(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(value) => Some(f64::from(*value)),
        Bson::Int64(value) => Some(*value as f64),
        Bson::Double(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

/// Convert BSON values to a finite `f64`, accepting numeric strings.
#[must_use]
pub fn bson_to_f64_loose(value: &Bson) -> Option<f64> {
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
    fn nested_accessors_reject_empty_paths_and_wrong_types() {
        let document = doc! { "outer": { "number": 10_i64, "name": "test" } };

        assert_eq!(nested_i64(&document, &[]), None);
        assert_eq!(nested_i64(&document, &["outer", "number"]), Some(10));
        assert_eq!(nested_str(&document, &["outer", "name"]), Some("test"));
        assert_eq!(nested_str(&document, &["outer", "number"]), None);
    }

    #[test]
    fn exact_i64_rejects_fractional_and_out_of_range_doubles() {
        assert_eq!(bson_to_i64_exact(&Bson::Double(56.0)), Some(56));
        assert_eq!(bson_to_i64_exact(&Bson::Double(56.8)), None);
        assert_eq!(bson_to_i64_exact(&Bson::Double(-(2_f64.powi(63)))), Some(i64::MIN));
        assert_eq!(bson_to_i64_exact(&Bson::Double(2_f64.powi(63))), None);
        assert_eq!(bson_to_i64_exact(&Bson::Double(f64::NAN)), None);
    }

    #[test]
    fn unsigned_conversion_rejects_negative_values() {
        assert_eq!(bson_to_u64(&Bson::Int64(12)), Some(12));
        assert_eq!(bson_to_u64(&Bson::Double(12.8)), Some(12));
        assert_eq!(bson_to_u64(&Bson::Int64(-1)), None);
    }

    #[test]
    fn exact_i32_rejects_out_of_range_values() {
        assert_eq!(bson_to_i32_exact(&Bson::Int64(i64::from(i32::MAX))), Some(i32::MAX));
        assert_eq!(bson_to_i32_exact(&Bson::Int64(i64::from(i32::MAX) + 1)), None);
    }

    #[test]
    fn integer_conversion_rejects_doubles() {
        assert_eq!(bson_integer_to_i64(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_integer_to_i64(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_integer_to_i64(&Bson::Double(34.0)), None);
    }

    #[test]
    fn loose_conversions_accept_trimmed_strings() {
        assert_eq!(bson_to_i64_loose(&Bson::String(" 56 ".to_owned())), Some(56));
        assert_eq!(bson_to_f64_loose(&Bson::String(" 56.8 ".to_owned())), Some(56.8));
        assert_eq!(bson_to_f64_loose(&Bson::String("nan".to_owned())), None);
    }
}
