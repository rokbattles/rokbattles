use mongodb::bson::DateTime;
use mongodb::bson::{Bson, Document, doc};

use crate::bson_utils::bson_to_f64_loose;

/// Normalizes an epoch timestamp to milliseconds.
/// Accepts values expressed in seconds, milliseconds, or microseconds.
pub(crate) fn normalize_timestamp_millis(value: f64) -> Option<i64> {
    let abs = value.abs();

    let normalized = if abs < 1e12 {
        value * 1000.0
    } else if abs >= 1e17 {
        value / 1e6
    } else if abs >= 1e14 {
        value / 1e3
    } else {
        value
    };

    if normalized.is_finite() && normalized >= i64::MIN as f64 && normalized <= i64::MAX as f64 {
        Some(normalized as i64)
    } else {
        None
    }
}

/// Normalize a BSON timestamp field to milliseconds.
///
/// This accepts number-like BSON values (including numeric strings), and then
/// normalizes seconds/milliseconds/microseconds into epoch milliseconds.
pub(crate) fn normalize_bson_timestamp_millis(value: Option<&Bson>) -> Option<i64> {
    let raw = bson_to_f64_loose(value?)?;
    normalize_timestamp_millis(raw)
}

/// Formats UTC epoch milliseconds as a `YYYY-MM-DD` date string.
pub(crate) fn date_key_utc(millis: i64) -> Option<String> {
    let rfc3339 = DateTime::from_millis(millis).try_to_rfc3339_string().ok()?;
    rfc3339.get(0..10).map(ToOwned::to_owned)
}

/// Builds a Mongo filter that matches timestamps stored in microseconds.
pub(crate) fn build_mail_time_match(start_millis: i64, end_millis: i64) -> Document {
    let start_micros = start_millis.saturating_mul(1000);
    let end_micros = end_millis.saturating_mul(1000);

    doc! {
        "metadata.mail_time": { "$gte": start_micros, "$lt": end_micros }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_seconds_millis_and_micros() {
        assert_eq!(
            normalize_timestamp_millis(1_739_960_800.0),
            Some(1_739_960_800_000)
        );
        assert_eq!(
            normalize_timestamp_millis(1_739_960_800_000.0),
            Some(1_739_960_800_000)
        );
        assert_eq!(
            normalize_timestamp_millis(1_739_960_800_000_000.0),
            Some(1_739_960_800_000)
        );
    }

    #[test]
    fn builds_date_key() {
        assert_eq!(
            date_key_utc(1_735_689_600_000),
            Some("2025-01-01".to_string())
        );
    }

    #[test]
    fn builds_mail_time_match_in_microseconds() {
        let filter = build_mail_time_match(1_000, 2_000);
        let expression = filter
            .get_document("metadata.mail_time")
            .expect("mail_time expression");
        assert_eq!(expression.get_i64("$gte").ok(), Some(1_000_000));
        assert_eq!(expression.get_i64("$lt").ok(), Some(2_000_000));
    }

    #[test]
    fn normalizes_bson_timestamp_millis_from_numeric_string() {
        assert_eq!(
            normalize_bson_timestamp_millis(Some(&Bson::String("1739960800".to_string()))),
            Some(1_739_960_800_000)
        );
    }
}
