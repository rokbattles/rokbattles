use mongodb::bson::DateTime;

/// Convert epoch-like values in seconds/millis/micros to milliseconds.
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

/// Format epoch milliseconds as `YYYY-MM-DD` in UTC.
pub(crate) fn date_key_utc(millis: i64) -> Option<String> {
    let rfc3339 = DateTime::from_millis(millis).try_to_rfc3339_string().ok()?;
    rfc3339.get(0..10).map(ToOwned::to_owned)
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
}
