#![forbid(unsafe_code)]

//! DRASTC scoring model by Davor (TKC) and ROK Battles

mod aggregate;
mod metrics;
mod reference;
mod theoretical;
mod weights;

use aggregate::BattleAggregate;
use reference::RecordMetrics;
pub use reference::{DrastcReferenceRanges, ReferenceRange};
use serde::Serialize;
pub use theoretical::TheoreticalValues;
use theoretical::theoretical_for_pairing;
use weights::weighted_overall;

pub(crate) const MIN_REFERENCE_RANGE: f64 = 0.000_000_001;

/// Battle sample used by the DRASTC model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleRecord {
    /// Battle duration in seconds.
    pub duration_seconds: f64,
    /// Perspective-side kill points.
    pub kill_points: f64,
    /// Opposing-side kill points.
    pub opponent_kill_points: f64,
    /// Dead units inflicted on the opposing side.
    pub opponent_dead: f64,
    /// Severely wounded units inflicted on the opposing side.
    pub opponent_severely_wounded: f64,
    /// Slightly wounded units inflicted on the opposing side.
    pub opponent_slightly_wounded: f64,
    /// Dead units received by the perspective side.
    pub sender_dead: f64,
    /// Severely wounded units received by the perspective side.
    pub sender_severely_wounded: f64,
    /// Slightly wounded units received by the perspective side.
    pub sender_slightly_wounded: f64,
    /// Healing done by the perspective side.
    pub sender_healing: f64,
}

/// DRASTC evaluator.
#[derive(Debug, Default)]
pub struct DrastcModel {
    aggregate: BattleAggregate,
    samples: Vec<RecordMetrics>,
    theoretical: TheoreticalValues,
    reference_ranges: Option<DrastcReferenceRanges>,
}

impl DrastcModel {
    /// Create an empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add battle samples to the model.
    pub fn push(&mut self, records: impl IntoIterator<Item = BattleRecord>) {
        for record in records {
            self.push_one(record);
        }
    }

    fn push_one(&mut self, record: BattleRecord) {
        self.aggregate.push(record);
        self.samples.push(RecordMetrics::from_record(record));
    }

    /// Set theoretical Rage/Assist values by pairing.
    ///
    /// Unknown Rage pairings default to zero; Assist sums known commander values.
    pub fn set_theoretical(&mut self, primary_commander_id: u32, secondary_commander_id: u32) {
        self.theoretical = theoretical_for_pairing(primary_commander_id, secondary_commander_id);
    }

    /// Return the number of battle samples in the model.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Use externally calculated reference ranges for percentile-based scoring.
    pub fn set_reference_ranges(&mut self, reference_ranges: DrastcReferenceRanges) {
        self.reference_ranges = Some(reference_ranges);
    }

    /// Evaluate all records
    pub fn evaluate(&self) -> Option<DrastcScore> {
        if self.samples.is_empty() {
            return None;
        }

        let references = self
            .reference_ranges
            .unwrap_or_else(|| DrastcReferenceRanges::from_population(&self.samples));
        let metrics = self.aggregate.metrics();

        let damage = references.damage.score_curved(metrics.damage_per_second, 0.55);
        let rage = self.theoretical.rage_score();
        let assist = self.theoretical.assist_score();
        let sustainability =
            references.sustainability.score_curved(metrics.sustainability_per_second, 0.55);
        let trade = CategoryScore::fixed_range(metrics.trade_ratio, 0.0, 2.0);
        let consistency = references.consistency.score(metrics.consistency_rate);

        let breakdown =
            DrastcCategories { damage, rage, assist, sustainability, trade, consistency };
        let overall = weighted_overall(&breakdown);

        Some(DrastcScore { samples: self.aggregate.sample_count(), breakdown, overall })
    }
}

/// Final DRASTC output.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrastcScore {
    /// Number of battle samples evaluated.
    pub samples: u64,
    /// Normalized category scores.
    pub breakdown: DrastcCategories,
    /// Weighted score on a 0-10 scale.
    pub overall: f64,
}

/// Normalized DRASTC category scores.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrastcCategories {
    /// Damage score.
    pub damage: CategoryScore,
    /// Rage score.
    pub rage: CategoryScore,
    /// Assist/support score.
    pub assist: CategoryScore,
    /// Sustainability score.
    pub sustainability: CategoryScore,
    /// Trade efficiency score.
    pub trade: CategoryScore,
    /// Consistency score.
    pub consistency: CategoryScore,
}

/// One category's metric value, reference range, and normalized score.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScore {
    /// Raw metric value.
    pub value: f64,
    /// P10 reference value.
    pub p10: f64,
    /// P90 reference value.
    pub p90: f64,
    /// Normalized score on a 0-10 scale.
    pub score: f64,
}

impl CategoryScore {
    pub(crate) fn fixed_zero() -> Self {
        Self { value: 0.0, p10: 0.0, p90: 0.0, score: 0.0 }
    }

    fn fixed_range(value: f64, p10: f64, p90: f64) -> Self {
        ReferenceRange::new(1, p10, p90).score(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kill_points: f64, opponent_kill_points: f64) -> BattleRecord {
        BattleRecord {
            duration_seconds: 100.0,
            kill_points,
            opponent_kill_points,
            opponent_dead: 10.0,
            opponent_severely_wounded: 20.0,
            opponent_slightly_wounded: 70.0,
            sender_dead: 0.0,
            sender_severely_wounded: 10.0,
            sender_slightly_wounded: 30.0,
            sender_healing: 5.0,
        }
    }

    #[test]
    fn evaluate_returns_none_when_no_records_have_been_added() {
        let model = DrastcModel::new();

        assert!(model.evaluate().is_none());
    }

    #[test]
    fn evaluate_keeps_rage_and_assist_at_zero() {
        let mut model = DrastcModel::new();
        model.push([record(200.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.rage.score, 0.0);
        assert_eq!(score.breakdown.assist.score, 0.0);
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_gang_gamchan_achilles() {
        let mut model = DrastcModel::new();
        model.set_theoretical(579, 575);
        model.push([record(200.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 5.47);
        assert_close(score.breakdown.assist.score, 3.39);
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_qin_zhuge_liang() {
        let mut model = DrastcModel::new();
        model.set_theoretical(509, 179);
        model.push([record(200.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 9.82);
        assert_close(score.breakdown.assist.score, 5.61);
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_zhuge_liang_prime_hermann() {
        let mut model = DrastcModel::new();
        model.set_theoretical(179, 187);
        model.push([record(200.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 6.83);
        assert_close(score.breakdown.assist.score, 8.29);
    }

    #[test]
    fn evaluate_scores_aggregate_against_record_distribution() {
        let mut model = DrastcModel::new();
        model.push([
            record(50.0, 100.0),
            BattleRecord {
                duration_seconds: 100.0,
                kill_points: 200.0,
                opponent_kill_points: 100.0,
                opponent_dead: 20.0,
                opponent_severely_wounded: 30.0,
                opponent_slightly_wounded: 100.0,
                sender_dead: 0.0,
                sender_severely_wounded: 10.0,
                sender_slightly_wounded: 20.0,
                sender_healing: 10.0,
            },
            BattleRecord {
                duration_seconds: 100.0,
                kill_points: 300.0,
                opponent_kill_points: 100.0,
                opponent_dead: 30.0,
                opponent_severely_wounded: 40.0,
                opponent_slightly_wounded: 130.0,
                sender_dead: 0.0,
                sender_severely_wounded: 5.0,
                sender_slightly_wounded: 5.0,
                sender_healing: 20.0,
            },
        ]);

        let score = model.evaluate().expect("score");

        assert!(score.breakdown.damage.p10 < score.breakdown.damage.p90);
        assert_eq!(score.samples, 3);
    }

    #[test]
    fn evaluate_curves_damage_and_sustainability_scores() {
        let mut model = DrastcModel::new();
        model.push([
            BattleRecord {
                duration_seconds: 100.0,
                kill_points: 100.0,
                opponent_kill_points: 100.0,
                opponent_dead: 0.0,
                opponent_severely_wounded: 0.0,
                opponent_slightly_wounded: 0.0,
                sender_dead: 0.0,
                sender_severely_wounded: 0.0,
                sender_slightly_wounded: 100.0,
                sender_healing: 0.0,
            },
            BattleRecord {
                duration_seconds: 100.0,
                kill_points: 100.0,
                opponent_kill_points: 100.0,
                opponent_dead: 0.0,
                opponent_severely_wounded: 0.0,
                opponent_slightly_wounded: 200.0,
                sender_dead: 0.0,
                sender_severely_wounded: 0.0,
                sender_slightly_wounded: 0.0,
                sender_healing: 0.0,
            },
        ]);

        let score = model.evaluate().expect("score");

        assert!(score.breakdown.damage.score > 5.0);
        assert!(score.breakdown.sustainability.score > 5.0);
    }

    #[test]
    fn evaluate_can_score_against_external_reference_ranges() {
        let reference_ranges = DrastcReferenceRanges {
            damage: ReferenceRange::new(2, 1.2, 2.8),
            sustainability: ReferenceRange::new(2, -1.0, 1.0),
            consistency: ReferenceRange::new(2, 0.0, 1.0),
        };

        let mut model = DrastcModel::new();
        model.set_reference_ranges(reference_ranges);
        model.push([BattleRecord {
            duration_seconds: 100.0,
            kill_points: 100.0,
            opponent_kill_points: 100.0,
            opponent_dead: 0.0,
            opponent_severely_wounded: 0.0,
            opponent_slightly_wounded: 200.0,
            sender_dead: 0.0,
            sender_severely_wounded: 0.0,
            sender_slightly_wounded: 0.0,
            sender_healing: 0.0,
        }]);

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.damage.p10, reference_ranges.damage.p10);
        assert_eq!(score.breakdown.damage.p90, reference_ranges.damage.p90);
        assert!(score.breakdown.damage.score > 5.0);
    }

    #[test]
    fn evaluate_infers_consistency_from_severe_dead_outcome() {
        let mut model = DrastcModel::new();
        model.push([record(50.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.consistency.value, 0.5);
    }

    #[test]
    fn evaluate_scores_equal_trade_ratio_as_five() {
        let mut model = DrastcModel::new();
        model.push([record(100.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.trade.score, 5.0);
    }

    #[test]
    fn evaluate_scores_double_trade_ratio_as_ten() {
        let mut model = DrastcModel::new();
        model.push([record(200.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.trade.score, 10.0);
    }

    #[test]
    fn push_adds_multiple_records() {
        let mut model = DrastcModel::new();
        model.push([record(100.0, 100.0), record(200.0, 100.0)]);

        let score = model.evaluate().expect("score");

        assert_eq!(score.samples, 2);
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 0.02, "actual={actual}, expected={expected}");
    }
}
