use chrono::{DateTime, Days, NaiveDate, Timelike, Utc};

use crate::{error::RokGtError, models::MemberDateRange};

const DATA_READY_HOUR_UTC: u32 = 2;

pub(crate) fn current_member_date_range() -> Result<MemberDateRange, RokGtError> {
    member_date_range_for(Utc::now())
}

pub(crate) fn member_date_range_for(now: DateTime<Utc>) -> Result<MemberDateRange, RokGtError> {
    let days_back = if now.hour() >= DATA_READY_HOUR_UTC { 1 } else { 2 };
    let data_date = now
        .date_naive()
        .checked_sub_days(Days::new(days_back))
        .ok_or(RokGtError::DateOutOfRange)?;
    let query_date = data_date.format("%Y-%m-%d").to_string();
    Ok(MemberDateRange::new(query_date.clone(), query_date))
}

pub(crate) fn date_to_iso_2_utc(value: &str) -> Result<String, RokGtError> {
    let date = NaiveDate::parse_from_str(value, "%Y/%m/%d")
        .or_else(|_| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .map_err(|_| RokGtError::InvalidDate(value.to_string()))?;
    Ok(format!("{}T02:00:00Z", date.format("%Y-%m-%d")))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn member_date_range_uses_yesterday_after_2_utc() {
        let now = Utc.with_ymd_and_hms(2026, 5, 25, 16, 0, 0).single().unwrap();

        let range = member_date_range_for(now).unwrap();

        assert_eq!(range.start, "2026-05-24");
        assert_eq!(range.end, "2026-05-24");
    }

    #[test]
    fn member_date_range_uses_two_days_ago_before_2_utc() {
        let now = Utc.with_ymd_and_hms(2026, 5, 26, 1, 59, 59).single().unwrap();

        let range = member_date_range_for(now).unwrap();

        assert_eq!(range.start, "2026-05-24");
        assert_eq!(range.end, "2026-05-24");
    }

    #[test]
    fn member_date_range_advances_at_2_utc() {
        let now = Utc.with_ymd_and_hms(2026, 5, 26, 2, 0, 0).single().unwrap();

        let range = member_date_range_for(now).unwrap();

        assert_eq!(range.start, "2026-05-25");
        assert_eq!(range.end, "2026-05-25");
    }

    #[test]
    fn date_to_iso_2_utc_normalizes_api_date() {
        let date = date_to_iso_2_utc("2026/05/24").unwrap();

        assert_eq!(date, "2026-05-24T02:00:00Z");
    }
}
