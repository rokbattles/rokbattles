use mongodb::bson::DateTime;

const LOOT_WINDOW_DAYS: i64 = 365;
const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1_000;

pub(crate) fn loot_cutoff_mail_time(run_at: DateTime) -> i64 {
    run_at
        .timestamp_millis()
        .saturating_sub(LOOT_WINDOW_DAYS * MILLIS_PER_DAY)
        .saturating_mul(1_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_preserves_utc_time_and_converts_to_microseconds() {
        let run_at =
            DateTime::parse_rfc3339_str("2026-10-05T12:34:56.789Z").expect("run timestamp");
        let expected = DateTime::parse_rfc3339_str("2025-10-05T12:34:56.789Z")
            .expect("cutoff timestamp")
            .timestamp_millis()
            * 1_000;

        assert_eq!(loot_cutoff_mail_time(run_at), expected);
    }

    #[test]
    fn cutoff_uses_365_days_across_a_leap_year() {
        let run_at = DateTime::parse_rfc3339_str("2024-03-01T08:00:00Z").expect("run timestamp");
        let expected = DateTime::parse_rfc3339_str("2023-03-02T08:00:00Z")
            .expect("cutoff timestamp")
            .timestamp_millis()
            * 1_000;

        assert_eq!(loot_cutoff_mail_time(run_at), expected);
    }
}
