use std::collections::HashMap;

use mongodb::bson::Bson;
use rokbattles_bson::{bson_to_f64_loose, bson_to_i64_loose};

use super::{
    store::RssMailDocument,
    types::{
        ResourceDailyResponse, ResourceDailyValueByTypeResponse, ResourceTotalsByTypeResponse,
        ResourceTotalsResponse,
    },
};
use crate::{
    routes::governor::date_range::GovernorDateRange,
    time_utils::{date_key_utc, normalize_bson_timestamp_millis},
};

#[derive(Debug, Clone)]
pub(crate) struct AggregatedResources {
    pub total_reports: i64,
    pub crystals_gain: ResourceTotalsResponse,
    pub resources: Vec<ResourceTotalsByTypeResponse>,
    pub daily: Vec<ResourceDailyResponse>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ResourceTotals {
    gain: i64,
    bonus: i64,
    total: i64,
}

impl ResourceTotals {
    fn add(&mut self, value: ResourceTotals) {
        self.gain += value.gain;
        self.bonus += value.bonus;
        self.total += value.total;
    }

    fn add_total_only(&mut self, total: i64) {
        self.gain += total;
        self.total += total;
    }

    fn into_response(self) -> ResourceTotalsResponse {
        ResourceTotalsResponse { gain: self.gain, bonus: self.bonus, total: self.total }
    }

    fn into_type_response(self, type_id: i64) -> ResourceTotalsByTypeResponse {
        ResourceTotalsByTypeResponse {
            type_id,
            gain: self.gain,
            bonus: self.bonus,
            total: self.total,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct DailyBucket {
    date: String,
    crystals_gain: i64,
    resources: HashMap<i64, i64>,
}

impl DailyBucket {
    fn new(date: String) -> Self {
        Self { date, crystals_gain: 0, resources: HashMap::new() }
    }

    fn add_crystals_gain(&mut self, value: i64) {
        self.crystals_gain += value;
    }

    fn add_resource_total(&mut self, type_id: i64, total: i64) {
        *self.resources.entry(type_id).or_default() += total;
    }

    fn into_response(self) -> ResourceDailyResponse {
        let mut resources = self
            .resources
            .into_iter()
            .map(|(type_id, total)| ResourceDailyValueByTypeResponse { type_id, total })
            .collect::<Vec<_>>();
        resources.sort_by_key(|left| left.type_id);

        ResourceDailyResponse { date: self.date, crystals_gain: self.crystals_gain, resources }
    }
}

pub(crate) fn aggregate_resources(
    mails: Vec<RssMailDocument>,
    range: &GovernorDateRange,
) -> AggregatedResources {
    let mut total_reports = 0;
    let mut crystals_gain = ResourceTotals::default();
    let mut resources: HashMap<i64, ResourceTotals> = HashMap::new();
    let mut daily_buckets: HashMap<String, DailyBucket> = HashMap::new();

    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata.as_ref().and_then(|metadata| metadata.mail_time.as_ref()),
        ) else {
            continue;
        };

        if event_time_millis < range.start_millis || event_time_millis >= range.end_millis {
            continue;
        }

        let Some(date_key) = date_key_utc(event_time_millis) else {
            continue;
        };

        let Some(rss) = mail.rss else {
            continue;
        };

        total_reports += 1;

        let day =
            daily_buckets.entry(date_key.clone()).or_insert_with(|| DailyBucket::new(date_key));

        if let Some(crystals_total) = extract_floor_non_negative_i64(rss.crystals_gain.as_ref())
            && crystals_total > 0
        {
            crystals_gain.add_total_only(crystals_total);
            day.add_crystals_gain(crystals_total);
        }

        let Some(resource_type) = rss.rss_type.as_ref().and_then(parse_i64_loose) else {
            continue;
        };

        if !is_supported_rss_resource_type(resource_type) {
            continue;
        }

        let Some(split) = split_from_values(rss.rss_value.as_ref(), rss.rss_bonus.as_ref()) else {
            continue;
        };
        if split.total == 0 {
            continue;
        }

        resources.entry(resource_type).or_default().add(split);
        day.add_resource_total(resource_type, split.total);
    }

    let mut resources = resources
        .into_iter()
        .map(|(type_id, totals)| totals.into_type_response(type_id))
        .collect::<Vec<_>>();
    resources.sort_by_key(|left| left.type_id);

    let mut daily = daily_buckets.into_values().map(DailyBucket::into_response).collect::<Vec<_>>();
    daily.sort_by(|left, right| left.date.cmp(&right.date));

    AggregatedResources {
        total_reports,
        crystals_gain: crystals_gain.into_response(),
        resources,
        daily,
    }
}

fn is_supported_rss_resource_type(resource_type: i64) -> bool {
    matches!(resource_type, 1..=5)
}

fn split_from_values(total: Option<&Bson>, bonus: Option<&Bson>) -> Option<ResourceTotals> {
    let total = extract_floor_non_negative_i64(total)?;
    let raw_bonus = extract_floor_non_negative_i64(bonus).unwrap_or(0);
    let capped_bonus = raw_bonus.min(total);

    Some(ResourceTotals { gain: total.saturating_sub(capped_bonus), bonus: capped_bonus, total })
}

fn extract_floor_non_negative_i64(value: Option<&Bson>) -> Option<i64> {
    let numeric = bson_to_f64_loose(value?)?;
    if numeric.is_sign_negative() {
        return None;
    }

    let floored = numeric.floor();
    if floored < i64::MIN as f64 || floored > i64::MAX as f64 {
        return None;
    }

    Some(floored as i64)
}

fn extract_event_time_millis(mail_time: Option<&Bson>) -> Option<i64> {
    normalize_bson_timestamp_millis(mail_time)
}

fn parse_i64_loose(value: &Bson) -> Option<i64> {
    bson_to_i64_loose(value)
}

#[cfg(test)]
mod tests {
    use super::{
        super::store::{MailMetadataDocument, RssSectionDocument},
        *,
    };

    #[test]
    fn split_from_values_floors_total_and_bonus_then_subtracts() {
        let split = split_from_values(Some(&Bson::Double(4104.32)), Some(&Bson::Double(232.0)))
            .expect("split");

        assert_eq!(split.total, 4104);
        assert_eq!(split.bonus, 232);
        assert_eq!(split.gain, 3872);
    }

    #[test]
    fn split_from_values_clamps_bonus_to_total() {
        let split =
            split_from_values(Some(&Bson::Double(2.12)), Some(&Bson::Int64(10))).expect("split");

        assert_eq!(split.total, 2);
        assert_eq!(split.bonus, 2);
        assert_eq!(split.gain, 0);
    }

    #[test]
    fn extract_event_time_supports_seconds_millis_and_micros() {
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800))),
            Some(1_739_960_800_000)
        );
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800_000))),
            Some(1_739_960_800_000)
        );
        assert_eq!(
            extract_event_time_millis(Some(&Bson::Int64(1_739_960_800_000_000))),
            Some(1_739_960_800_000)
        );
    }

    #[test]
    fn aggregate_resources_returns_crystals_field_and_id_resources() {
        let range = GovernorDateRange {
            start_millis: 1_735_689_600_000,
            end_millis: 1_735_862_400_000,
            start: "2025-01-01".to_string(),
            end: "2025-01-02".to_string(),
        };

        let first_day_time_micros = 1_735_689_600_000_000;
        let second_day_time_micros = 1_735_776_000_000_000;

        let mails = vec![
            build_mail(first_day_time_micros, 1, 4104.32, 232.0, 19.9),
            build_mail(first_day_time_micros, 5, 2.12, 0.0, 1.0),
            build_mail(second_day_time_micros, 4, 1850.2, 255.0, 3.0),
            build_mail(second_day_time_micros, 99, 999.0, 50.0, 0.0),
        ];

        let aggregated = aggregate_resources(mails, &range);

        assert_eq!(aggregated.total_reports, 4);
        assert_eq!(aggregated.crystals_gain.total, 23);
        assert_eq!(aggregated.crystals_gain.bonus, 0);
        assert_eq!(aggregated.crystals_gain.gain, 23);

        let type_1 = find_resource(&aggregated.resources, 1).expect("resource type 1");
        assert_eq!(type_1.total, 4104);
        assert_eq!(type_1.bonus, 232);
        assert_eq!(type_1.gain, 3872);

        let type_5 = find_resource(&aggregated.resources, 5).expect("resource type 5");
        assert_eq!(type_5.total, 2);
        assert_eq!(type_5.bonus, 0);
        assert_eq!(type_5.gain, 2);

        let type_4 = find_resource(&aggregated.resources, 4).expect("resource type 4");
        assert_eq!(type_4.total, 1850);
        assert_eq!(type_4.bonus, 255);
        assert_eq!(type_4.gain, 1595);

        assert_eq!(aggregated.daily.len(), 2);
        assert_eq!(aggregated.daily[0].date, "2025-01-01");
        assert_eq!(aggregated.daily[0].crystals_gain, 20);
        assert_eq!(find_daily_total(&aggregated.daily[0], 1), Some(4104));
        assert_eq!(find_daily_total(&aggregated.daily[0], 5), Some(2));

        assert_eq!(aggregated.daily[1].date, "2025-01-02");
        assert_eq!(aggregated.daily[1].crystals_gain, 3);
        assert_eq!(find_daily_total(&aggregated.daily[1], 4), Some(1850));
    }

    fn find_resource(
        resources: &[ResourceTotalsByTypeResponse],
        type_id: i64,
    ) -> Option<&ResourceTotalsByTypeResponse> {
        resources.iter().find(|entry| entry.type_id == type_id)
    }

    fn find_daily_total(day: &ResourceDailyResponse, type_id: i64) -> Option<i64> {
        day.resources.iter().find(|entry| entry.type_id == type_id).map(|entry| entry.total)
    }

    fn build_mail(
        mail_time: i64,
        rss_type: i64,
        rss_value: f64,
        rss_bonus: f64,
        crystals_gain: f64,
    ) -> RssMailDocument {
        RssMailDocument {
            metadata: Some(MailMetadataDocument { mail_time: Some(Bson::Int64(mail_time)) }),
            rss: Some(RssSectionDocument {
                rss_type: Some(Bson::Int64(rss_type)),
                rss_value: Some(Bson::Double(rss_value)),
                rss_bonus: Some(Bson::Double(rss_bonus)),
                crystals_gain: Some(Bson::Double(crystals_gain)),
            }),
        }
    }
}
