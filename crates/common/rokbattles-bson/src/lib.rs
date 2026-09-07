#![forbid(unsafe_code)]

//! Reads nested BSON fields and converts numeric values.
//!
//! These helpers operate on [`Document`] and [`Bson`] values in memory. They
//! return `None` when a path cannot be followed or a conversion is unsupported,
//! leaving the caller to choose a default or report an error.
//!
//! # Paths
//!
//! Paths contain literal field names. `&["body", "count"]` follows two fields;
//! `&["body.count"]` looks up a single key containing a dot. Intermediate values
//! must be documents: paths do not index arrays or implement MongoDB query syntax.
//!
//! An empty path returns the input document from [`nested_document`]. All other
//! nested accessors require a final field name and return `None` for an empty
//! path. Borrowed results refer to the original document; [`nested_string`]
//! instead copies the string.
//!
//! # Numeric conversions
//!
//! Numeric helpers support `Int32`, `Int64`, and, where indicated, finite
//! `Double` values. Other BSON types, including `Decimal128`, are rejected.
//! Only the loose helpers accept strings, trimming whitespace before parsing.
//!
//! | Helper | Conversion |
//! | --- | --- |
//! | [`bson_integer_to_i64`] | Integer storage types only; rejects doubles. |
//! | [`bson_to_i64`] | Integers unchanged; in-range doubles truncated toward zero. |
//! | [`bson_to_i64_exact`] | Integers unchanged; doubles must be whole and in range. |
//! | [`bson_to_i32_exact`] | Exact `i64` conversion, then an `i32` range check. |
//! | [`bson_to_u64`] | Truncating `i64` conversion, then a non-negative check. |
//! | [`bson_to_i64_loose`] | Exact `i64` conversion, also accepting integer strings. |
//! | [`bson_to_f64`] | Finite doubles and integers; large `Int64` values may round. |
//! | [`bson_to_f64_loose`] | Finite `f64` conversion, also accepting numeric strings. |
//!
//! Double-to-`i64` conversions require `-2^63 <= value < 2^63` before casting.
//! Exact conversion preserves the stored double's integer value; it cannot
//! recover precision lost before that double was stored. The `u64` helper also
//! uses this signed range rather than the full unsigned range.
//!
//! # Examples
//!
//! Choose whether fractional values are acceptable for the field being read:
//!
//! ```
//! use mongodb::bson::doc;
//! use rokbattles_bson::{nested_i64, nested_i64_exact, nested_str};
//!
//! let document = doc! { "body": { "count": 12.8, "name": "Battle" } };
//! assert_eq!(nested_i64(&document, &["body", "count"]), Some(12));
//! assert_eq!(nested_i64_exact(&document, &["body", "count"]), None);
//! assert_eq!(nested_str(&document, &["body", "name"]), Some("Battle"));
//! ```

use mongodb::bson::{Bson, Document};

/// Borrows the value at a nonempty path of literal field names.
///
/// Returns `None` for an empty path, a missing key, or a non-document
/// intermediate value. The final value may have any BSON type, including null.
/// See the [path rules](crate#paths) for dotted keys and arrays.
#[must_use]
pub fn nested_value<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Bson> {
    // Only intermediate values must be documents; the final key can hold any BSON type.
    let (key, parents) = path.split_last()?;
    let parent = nested_document(document, parents)?;
    parent.get(*key)
}

/// Borrows the document at a path of literal field names.
///
/// An empty path returns `document` itself. Returns `None` if any key is missing
/// or any value along the path, including the final value, is not a document.
#[must_use]
pub fn nested_document<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Document> {
    path.iter().try_fold(document, |current, key| current.get_document(*key).ok())
}

/// Reads a nested number as `i64`, truncating doubles toward zero.
///
/// Combines [`nested_value`] with [`bson_to_i64`]. Returns `None` if lookup or
/// conversion fails; strings are not parsed.
#[must_use]
pub fn nested_i64(document: &Document, path: &[&str]) -> Option<i64> {
    nested_value(document, path).and_then(bson_to_i64)
}

/// Reads a nested number as `i64`, rejecting fractional doubles.
///
/// Combines [`nested_value`] with [`bson_to_i64_exact`]. Returns `None` if lookup
/// or conversion fails. Useful for identifiers and counts that must be whole.
#[must_use]
pub fn nested_i64_exact(document: &Document, path: &[&str]) -> Option<i64> {
    nested_value(document, path).and_then(bson_to_i64_exact)
}

/// Reads a nested number as a finite `f64`.
///
/// Combines [`nested_value`] with [`bson_to_f64`], including its possible
/// rounding of large integers. Returns `None` if lookup or conversion fails.
#[must_use]
pub fn nested_f64(document: &Document, path: &[&str]) -> Option<f64> {
    nested_value(document, path).and_then(bson_to_f64)
}

/// Reads a nested BSON boolean without coercing numbers or strings.
///
/// Returns `None` if [`nested_value`] fails or the value is not `Bson::Boolean`.
#[must_use]
pub fn nested_bool(document: &Document, path: &[&str]) -> Option<bool> {
    nested_value(document, path).and_then(Bson::as_bool)
}

/// Borrows a nested BSON string.
///
/// Returns `None` if [`nested_value`] fails or the value is not `Bson::String`.
/// Other BSON types are not formatted as text.
#[must_use]
pub fn nested_str<'a>(document: &'a Document, path: &[&str]) -> Option<&'a str> {
    nested_value(document, path).and_then(Bson::as_str)
}

/// Copies a nested BSON string into an owned `String`.
///
/// Returns `None` under the same conditions as [`nested_str`]. Use that helper
/// when the result can borrow from the document.
#[must_use]
pub fn nested_string(document: &Document, path: &[&str]) -> Option<String> {
    nested_str(document, path).map(ToOwned::to_owned)
}

/// Borrows a nested BSON array, preserving element order.
///
/// Returns `None` if [`nested_value`] fails or the value is not `Bson::Array`.
/// An empty array is returned as `Some`; element types are not checked.
#[must_use]
pub fn nested_array<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Vec<Bson>> {
    nested_value(document, path).and_then(Bson::as_array)
}

/// Reads `Int32` or `Int64` storage as `i64`.
///
/// Returns `None` for every other variant, including whole-valued doubles.
/// Use [`bson_to_i64_exact`] when whole doubles should also be accepted.
#[must_use]
pub fn bson_integer_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        _ => None,
    }
}

/// Converts `Int32`, `Int64`, or an in-range finite `Double` to `i64`.
///
/// Doubles are truncated toward zero after checking the interval
/// `-2^63 <= value < 2^63`. Returns `None` for non-finite or out-of-range doubles
/// and unsupported variants. Strings are not parsed.
///
/// # Examples
///
/// ```
/// use mongodb::bson::Bson;
/// use rokbattles_bson::bson_to_i64;
///
/// assert_eq!(bson_to_i64(&Bson::Double(12.8)), Some(12));
/// assert_eq!(bson_to_i64(&Bson::Double(-12.8)), Some(-12));
/// ```
#[must_use]
pub fn bson_to_i64(value: &Bson) -> Option<i64> {
    // `i64::MAX as f64` rounds up to 2^63. Use that exclusive upper bound so
    // an out-of-range double cannot pass the check and saturate during the cast.
    match value {
        Bson::Double(value)
            if value.is_finite() && *value >= i64::MIN as f64 && *value < -(i64::MIN as f64) =>
        {
            Some(*value as i64)
        }
        _ => bson_integer_to_i64(value),
    }
}

/// Converts through [`bson_to_i64`] and accepts a non-negative result.
///
/// The result is limited to `0..=i64::MAX`, not the full `u64` range. Doubles
/// are truncated before the sign check, so values strictly between -1 and 0
/// become zero. Returns `None` if the signed conversion fails or remains negative.
#[must_use]
pub fn bson_to_u64(value: &Bson) -> Option<u64> {
    u64::try_from(bson_to_i64(value)?).ok()
}

/// Converts integer storage or a whole finite `Double` to `i64`.
///
/// Doubles must have no fractional part and lie in `-2^63 <= value < 2^63`.
/// Returns `None` for all other values, including strings. Both signs of
/// floating-point zero become integer zero.
#[must_use]
pub fn bson_to_i64_exact(value: &Bson) -> Option<i64> {
    // Whole-valued doubles still need the exclusive 2^63 bound used by `bson_to_i64`.
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

/// Converts with [`bson_to_i64_exact`], then checks the `i32` range.
///
/// Returns `None` if either conversion fails; values are neither truncated
/// nor clamped to fit.
#[must_use]
pub fn bson_to_i32_exact(value: &Bson) -> Option<i32> {
    i32::try_from(bson_to_i64_exact(value)?).ok()
}

/// Converts with [`bson_to_i64_exact`], also accepting decimal integer strings.
///
/// Strings are trimmed, then parsed as `i64`. Fractional text such as `"12.0"`,
/// exponent notation such as `"1e2"`, and out-of-range integers return `None`.
/// A BSON double of `12.0` is accepted because it follows the numeric path.
#[must_use]
pub fn bson_to_i64_loose(value: &Bson) -> Option<i64> {
    bson_to_i64_exact(value).or_else(|| match value {
        Bson::String(raw) => raw.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// Converts `Int32`, `Int64`, or a finite `Double` to `f64`.
///
/// Every `Int32` is represented exactly; large `Int64` values may lose precision.
/// Finite doubles are returned unchanged. Returns `None` for non-finite doubles
/// or unsupported variants, including strings and `Decimal128`.
#[must_use]
pub fn bson_to_f64(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(value) => Some(f64::from(*value)),
        Bson::Int64(value) => Some(*value as f64),
        Bson::Double(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

/// Converts with [`bson_to_f64`], also parsing trimmed strings as `f64`.
///
/// Strings may contain fractions or exponent notation. Returns `None` if
/// parsing fails or produces a non-finite value, including NaN and infinity.
/// Numeric conversions retain the rounding behavior of [`bson_to_f64`].
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
