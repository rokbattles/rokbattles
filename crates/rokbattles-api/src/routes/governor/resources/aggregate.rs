use std::collections::HashMap;

use mongodb::bson::Bson;

use super::store::RssMailDocument;
use super::types::{ResourceBreakdownResponse, ResourceDailyResponse, ResourceTotalsResponse};
use crate::bson_utils::{bson_to_f64_loose, bson_to_i64_loose};
use crate::routes::governor::date_range::GovernorDateRange;
use crate::time_utils::{date_key_utc, normalize_bson_timestamp_millis};

#[derive(Debug, Clone)]
pub(crate) struct AggregatedResources {
    pub total_reports: i64,
    pub breakdown: ResourceBreakdownResponse,
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
        ResourceTotalsResponse {
            gain: self.gain,
            bonus: self.bonus,
            total: self.total,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ResourceKind {
    Food,
    Wood,
    Stone,
    Gold,
    Gems,
}

#[derive(Debug, Clone, Copy, Default)]
struct ResourceBreakdown {
    crystals: ResourceTotals,
    food: ResourceTotals,
    wood: ResourceTotals,
    stone: ResourceTotals,
    gold: ResourceTotals,
    gems: ResourceTotals,
}

impl ResourceBreakdown {
    fn add_split(&mut self, resource: ResourceKind, split: ResourceTotals) {
        match resource {
            ResourceKind::Food => self.food.add(split),
            ResourceKind::Wood => self.wood.add(split),
            ResourceKind::Stone => self.stone.add(split),
            ResourceKind::Gold => self.gold.add(split),
            ResourceKind::Gems => self.gems.add(split),
        }
    }

    fn add_crystals_total(&mut self, total: i64) {
        self.crystals.add_total_only(total);
    }

    fn into_response(self) -> ResourceBreakdownResponse {
        ResourceBreakdownResponse {
            crystals: self.crystals.into_response(),
            food: self.food.into_response(),
            wood: self.wood.into_response(),
            stone: self.stone.into_response(),
            gold: self.gold.into_response(),
            gems: self.gems.into_response(),
        }
    }
}

#[derive(Debug, Clone)]
struct DailyBucket {
    date: String,
    crystals: i64,
    food: i64,
    wood: i64,
    stone: i64,
    gold: i64,
    gems: i64,
}

impl DailyBucket {
    fn new(date: String) -> Self {
        Self {
            date,
            crystals: 0,
            food: 0,
            wood: 0,
            stone: 0,
            gold: 0,
            gems: 0,
        }
    }

    fn add_total(&mut self, resource: ResourceKind, total: i64) {
        match resource {
            ResourceKind::Food => self.food += total,
            ResourceKind::Wood => self.wood += total,
            ResourceKind::Stone => self.stone += total,
            ResourceKind::Gold => self.gold += total,
            ResourceKind::Gems => self.gems += total,
        }
    }

    fn add_crystals_total(&mut self, total: i64) {
        self.crystals += total;
    }

    fn into_response(self) -> ResourceDailyResponse {
        ResourceDailyResponse {
            date: self.date,
            crystals: self.crystals,
            food: self.food,
            wood: self.wood,
            stone: self.stone,
            gold: self.gold,
            gems: self.gems,
        }
    }
}

pub(crate) fn aggregate_resources(
    mails: Vec<RssMailDocument>,
    range: &GovernorDateRange,
) -> AggregatedResources {
    let mut total_reports = 0;
    let mut breakdown = ResourceBreakdown::default();
    let mut daily_buckets: HashMap<String, DailyBucket> = HashMap::new();

    for mail in mails {
        let Some(event_time_millis) = extract_event_time_millis(
            mail.metadata
                .as_ref()
                .and_then(|metadata| metadata.mail_time.as_ref()),
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

        let daily_bucket = daily_buckets
            .entry(date_key.clone())
            .or_insert_with(|| DailyBucket::new(date_key));

        if let Some(crystals_total) = extract_floor_non_negative_i64(rss.crystals_gain.as_ref()) {
            breakdown.add_crystals_total(crystals_total);
            daily_bucket.add_crystals_total(crystals_total);
        }

        let Some(resource_kind) = rss
            .rss_type
            .as_ref()
            .and_then(parse_i64_loose)
            .and_then(resource_from_type)
        else {
            continue;
        };

        let Some(split) = split_from_values(rss.rss_value.as_ref(), rss.rss_bonus.as_ref()) else {
            continue;
        };

        breakdown.add_split(resource_kind, split);
        daily_bucket.add_total(resource_kind, split.total);
    }

    let mut daily = daily_buckets
        .into_values()
        .map(DailyBucket::into_response)
        .collect::<Vec<_>>();
    daily.sort_by(|left, right| left.date.cmp(&right.date));

    AggregatedResources {
        total_reports,
        breakdown: breakdown.into_response(),
        daily,
    }
}

fn resource_from_type(resource_type: i64) -> Option<ResourceKind> {
    match resource_type {
        1 => Some(ResourceKind::Food),
        2 => Some(ResourceKind::Wood),
        3 => Some(ResourceKind::Stone),
        4 => Some(ResourceKind::Gold),
        5 => Some(ResourceKind::Gems),
        _ => None,
    }
}

fn split_from_values(total: Option<&Bson>, bonus: Option<&Bson>) -> Option<ResourceTotals> {
    let total = extract_floor_non_negative_i64(total)?;
    let raw_bonus = extract_floor_non_negative_i64(bonus).unwrap_or(0);
    let capped_bonus = raw_bonus.min(total);

    Some(ResourceTotals {
        gain: total.saturating_sub(capped_bonus),
        bonus: capped_bonus,
        total,
    })
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
    use super::super::store::{MailMetadataDocument, RssSectionDocument};
    use super::*;

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
    fn aggregate_resources_includes_crystals_and_gems() {
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
        assert_eq!(aggregated.breakdown.crystals.total, 23);
        assert_eq!(aggregated.breakdown.crystals.bonus, 0);
        assert_eq!(aggregated.breakdown.crystals.gain, 23);

        assert_eq!(aggregated.breakdown.food.total, 4104);
        assert_eq!(aggregated.breakdown.food.bonus, 232);
        assert_eq!(aggregated.breakdown.food.gain, 3872);

        assert_eq!(aggregated.breakdown.gems.total, 2);
        assert_eq!(aggregated.breakdown.gems.bonus, 0);
        assert_eq!(aggregated.breakdown.gems.gain, 2);

        assert_eq!(aggregated.breakdown.gold.total, 1850);
        assert_eq!(aggregated.breakdown.gold.bonus, 255);
        assert_eq!(aggregated.breakdown.gold.gain, 1595);

        assert_eq!(aggregated.daily.len(), 2);
        assert_eq!(aggregated.daily[0].date, "2025-01-01");
        assert_eq!(aggregated.daily[0].food, 4104);
        assert_eq!(aggregated.daily[0].gems, 2);
        assert_eq!(aggregated.daily[0].crystals, 20);

        assert_eq!(aggregated.daily[1].date, "2025-01-02");
        assert_eq!(aggregated.daily[1].gold, 1850);
        assert_eq!(aggregated.daily[1].crystals, 3);
    }

    fn build_mail(
        mail_time: i64,
        rss_type: i64,
        rss_value: f64,
        rss_bonus: f64,
        crystals_gain: f64,
    ) -> RssMailDocument {
        RssMailDocument {
            metadata: Some(MailMetadataDocument {
                mail_time: Some(Bson::Int64(mail_time)),
            }),
            rss: Some(RssSectionDocument {
                rss_type: Some(Bson::Int64(rss_type)),
                rss_value: Some(Bson::Double(rss_value)),
                rss_bonus: Some(Bson::Double(rss_bonus)),
                crystals_gain: Some(Bson::Double(crystals_gain)),
            }),
        }
    }
}
