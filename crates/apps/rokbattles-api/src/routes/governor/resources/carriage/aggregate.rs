//! Builds transfer totals, rankings, and consecutive activity groups.

use std::{cmp::Reverse, collections::BTreeMap, num::NonZeroUsize};

use rustc_hash::FxHashMap;
use serde::Serialize;

const PAGE_SIZE: usize = 15;
const TOP_COUNTERPARTIES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Direction {
    Received,
    Sent,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct ResourceAmount {
    #[serde(rename = "type")]
    pub kind: i64,
    pub amount: i64,
}

pub(super) struct TransferReport {
    pub mail_id: String,
    pub timestamp_millis: i64,
    pub raw_timestamp: i64,
    pub governor_id: i64,
    pub name: String,
    pub direction: Direction,
    pub resources: Vec<ResourceAmount>,
    pub tax: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Counterparty {
    governor_id: i64,
    name: String,
    resources: Vec<ResourceAmount>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityRow {
    id: String,
    direction: Direction,
    #[serde(flatten)]
    counterparty: Counterparty,
    newest_at: i64,
    oldest_at: i64,
    report_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    tax: Option<TaxRange>,
}

#[derive(Serialize)]
struct TaxRange {
    min: f64,
    max: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CarriageResponse {
    total_received: Vec<ResourceAmount>,
    total_sent: Vec<ResourceAmount>,
    top_received: Vec<Counterparty>,
    top_sent: Vec<Counterparty>,
    activity: Vec<ActivityRow>,
    page: usize,
    total_pages: usize,
    total_groups: usize,
}

#[derive(Default)]
struct ResourceTotals {
    by_kind: BTreeMap<i64, i64>,
}

impl ResourceTotals {
    fn add(&mut self, kind: i64, amount: i64) {
        let total = self.by_kind.entry(kind).or_default();
        *total = total.saturating_add(amount);
    }

    fn total(&self) -> i64 {
        self.by_kind.values().copied().fold(0, i64::saturating_add)
    }

    fn into_amounts(self) -> Vec<ResourceAmount> {
        self.by_kind.into_iter().map(|(kind, amount)| ResourceAmount { kind, amount }).collect()
    }
}

#[derive(Default)]
struct CounterpartyTotals {
    name: String,
    resources: ResourceTotals,
}

impl CounterpartyTotals {
    fn add(&mut self, report: &TransferReport) {
        if self.name.is_empty() && !report.name.trim().is_empty() {
            self.name = report.name.clone();
        }

        for resource in &report.resources {
            self.resources.add(resource.kind, resource.amount);
        }
    }

    fn into_counterparty(self, governor_id: i64) -> Counterparty {
        Counterparty { governor_id, name: self.name, resources: self.resources.into_amounts() }
    }
}

struct TransferGroup {
    id: String,
    governor_id: i64,
    direction: Direction,
    oldest_at: i64,
    newest_at: i64,
    report_count: usize,
    totals: CounterpartyTotals,
    tax: Option<TaxRange>,
}

impl TransferGroup {
    fn new(report: &TransferReport) -> Self {
        Self {
            id: report.mail_id.clone(),
            governor_id: report.governor_id,
            direction: report.direction,
            oldest_at: report.timestamp_millis,
            newest_at: report.timestamp_millis,
            report_count: 0,
            totals: CounterpartyTotals::default(),
            tax: None,
        }
    }

    fn add(&mut self, report: &TransferReport) {
        self.report_count += 1;
        self.oldest_at = report.timestamp_millis;
        self.totals.add(report);

        if let Some(rate) = report.tax {
            let range = self.tax.get_or_insert(TaxRange { min: rate, max: rate });
            range.min = range.min.min(rate);
            range.max = range.max.max(rate);
        }
    }

    fn into_activity(self) -> ActivityRow {
        ActivityRow {
            id: self.id,
            direction: self.direction,
            counterparty: self.totals.into_counterparty(self.governor_id),
            newest_at: self.newest_at,
            oldest_at: self.oldest_at,
            report_count: self.report_count,
            tax: self.tax,
        }
    }
}

/// Totals and ranks all reports before selecting a page of activity.
///
/// Only consecutive reports with the same counterparty and direction share a
/// group. Activity is ordered newest first.
pub(super) fn summarize(
    mut reports: Vec<TransferReport>,
    requested_page: NonZeroUsize,
) -> CarriageResponse {
    // Keep the raw timestamp as a tie-breaker for reports within the same
    // millisecond. The mail ID makes the order independent of the query order.
    reports.sort_unstable_by(|a, b| {
        (b.timestamp_millis, b.raw_timestamp, &b.mail_id).cmp(&(
            a.timestamp_millis,
            a.raw_timestamp,
            &a.mail_id,
        ))
    });

    let mut received = FxHashMap::<i64, CounterpartyTotals>::default();
    let mut sent = FxHashMap::<i64, CounterpartyTotals>::default();
    let mut groups = Vec::new();
    let mut current_group = None;

    for report in &reports {
        let counterparties = match report.direction {
            Direction::Received => &mut received,
            Direction::Sent => &mut sent,
        };
        counterparties.entry(report.governor_id).or_default().add(report);

        let group = current_group.get_or_insert_with(|| TransferGroup::new(report));
        if group.governor_id != report.governor_id || group.direction != report.direction {
            groups.push(std::mem::replace(group, TransferGroup::new(report)));
        }
        group.add(report);
    }

    groups.extend(current_group);

    let total_groups = groups.len();
    let total_pages = total_groups.div_ceil(PAGE_SIZE);
    let page = requested_page.get().min(total_pages.max(1));
    let activity = groups
        .into_iter()
        .skip((page - 1) * PAGE_SIZE)
        .take(PAGE_SIZE)
        .map(TransferGroup::into_activity)
        .collect();

    CarriageResponse {
        total_received: sum_resources(&received),
        total_sent: sum_resources(&sent),
        top_received: top_counterparties(received),
        top_sent: top_counterparties(sent),
        activity,
        page,
        total_pages,
        total_groups,
    }
}

fn sum_resources(counterparties: &FxHashMap<i64, CounterpartyTotals>) -> Vec<ResourceAmount> {
    let mut combined = ResourceTotals::default();

    for counterparty in counterparties.values() {
        for (&kind, &amount) in &counterparty.resources.by_kind {
            combined.add(kind, amount);
        }
    }

    combined.into_amounts()
}

fn top_counterparties(totals: FxHashMap<i64, CounterpartyTotals>) -> Vec<Counterparty> {
    let mut ranked: Vec<_> = totals
        .into_iter()
        .filter(|(_, totals)| !totals.resources.by_kind.is_empty())
        .map(|(id, totals)| (totals.resources.total(), id, totals))
        .collect();

    ranked.sort_unstable_by_key(|(amount, id, _)| (Reverse(*amount), *id));

    ranked
        .into_iter()
        .take(TOP_COUNTERPARTIES)
        .map(|(_, id, totals)| totals.into_counterparty(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(time: i64, governor_id: i64, direction: Direction, amount: i64) -> TransferReport {
        TransferReport {
            mail_id: time.to_string(),
            timestamp_millis: time,
            raw_timestamp: time,
            governor_id,
            name: String::new(),
            direction,
            resources: vec![ResourceAmount { kind: 2, amount }],
            tax: None,
        }
    }

    #[test]
    fn activity_combines_only_consecutive_reports_with_the_same_person_and_direction() {
        let reports = vec![
            report(1, 1, Direction::Sent, 3),
            report(2, 1, Direction::Received, 20),
            report(3, 2, Direction::Received, 4),
            report(5, 1, Direction::Received, 10),
            report(4, 1, Direction::Received, 15),
        ];

        let result = summarize(reports, NonZeroUsize::MIN);

        assert_eq!(result.total_groups, 4);
        assert_eq!(result.activity[0].report_count, 2);
        assert_eq!(result.activity[0].newest_at, 5);
        assert_eq!(result.activity[0].oldest_at, 4);
        assert_eq!(result.activity[0].counterparty.resources[0].amount, 25);
        assert_eq!(result.activity[2].counterparty.resources[0].amount, 20);
        assert_eq!(result.activity[3].direction, Direction::Sent);
        assert_eq!(result.top_received[0].resources[0].amount, 45);
        assert_eq!(result.top_sent[0].resources[0].amount, 3);
    }

    #[test]
    fn ranks_five_counterparties_per_direction_and_paginates_groups() {
        let reports = (1..=17_i64).map(|id| report(id, id, Direction::Received, id)).collect();

        let result = summarize(reports, NonZeroUsize::new(2).expect("page"));

        assert_eq!((result.page, result.total_pages, result.total_groups), (2, 2, 17));
        assert_eq!(result.activity.len(), 2);
        assert_eq!(result.activity[0].id, "2");
        assert_eq!(
            result.top_received.iter().map(|row| row.governor_id).collect::<Vec<_>>(),
            [17, 16, 15, 14, 13]
        );
        assert!(result.top_sent.is_empty());
    }

    #[test]
    fn resource_totals_include_counterparties_on_any_page_and_keep_directions_separate() {
        let mut reports: Vec<_> =
            (1..=17).map(|id| report(id, id, Direction::Received, id)).collect();
        let mut sent = report(18, 1, Direction::Sent, 5);
        sent.resources.push(ResourceAmount { kind: 4, amount: 10 });
        reports.push(sent);

        let result = summarize(reports, NonZeroUsize::new(2).expect("page"));

        assert_eq!(result.total_received, [ResourceAmount { kind: 2, amount: 153 }]);
        assert_eq!(
            result.total_sent,
            [ResourceAmount { kind: 2, amount: 5 }, ResourceAmount { kind: 4, amount: 10 }]
        );
    }

    #[test]
    fn activity_reports_the_range_of_recorded_tax_rates() {
        let mut first = report(4, 1, Direction::Sent, 10);
        first.tax = Some(0.125);
        let mut last = report(2, 1, Direction::Sent, 5);
        last.tax = Some(0.0625);

        let result =
            summarize(vec![first, report(3, 1, Direction::Sent, 20), last], NonZeroUsize::MIN);

        let tax = result.activity[0].tax.as_ref().expect("recorded rates");
        assert_eq!((tax.min, tax.max), (0.0625, 0.125));
    }
}
