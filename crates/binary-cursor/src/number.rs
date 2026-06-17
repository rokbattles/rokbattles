//! Number handling for float-backed values.

use serde_json::{Number, Value};

use crate::error::DecodeError;

pub(crate) fn number_value(value: f64) -> Result<Value, DecodeError> {
    if value.is_finite() {
        Ok(Value::Number(normalize_number(value)))
    } else {
        Err(DecodeError::NonFiniteNumber { value })
    }
}

fn normalize_number(value: f64) -> Number {
    if value == 0.0 {
        return Number::from(0);
    }

    if value.fract() == 0.0 {
        if value.is_sign_positive() {
            if let Some(int) = to_u64_exact(value) {
                return Number::from(int);
            }
        } else if let Some(int) = to_i64_exact(value) {
            return Number::from(int);
        }
    }

    number_from_f64(value)
}

fn number_from_f64(value: f64) -> Number {
    if let Some(number) = Number::from_f64(value) {
        number
    } else {
        // `number_value` rejects NaN and infinity before this helper is called.
        Number::from(0)
    }
}

fn to_u64_exact(value: f64) -> Option<u64> {
    if value < 0.0 || value > u64::MAX as f64 {
        return None;
    }
    let int = value as u64;
    if (int as f64) == value { Some(int) } else { None }
}

fn to_i64_exact(value: f64) -> Option<i64> {
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return None;
    }
    let int = value as i64;
    if (int as f64) == value { Some(int) } else { None }
}
