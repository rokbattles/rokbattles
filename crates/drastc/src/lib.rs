#![forbid(unsafe_code)]

//! DRASTC scoring model by Davor (TKC) and ROK Battles

mod aggregate;
mod metrics;
mod reference;
mod theoretical;
mod weights;

use aggregate::BattleAggregate;
pub use reference::{DrastcReferenceRanges, ReferenceRange};
use serde::Serialize;
pub use theoretical::TheoreticalValues;
use theoretical::{is_supported_pairing, theoretical_for_pairing};
use weights::weighted_overall;

pub(crate) const MIN_REFERENCE_RANGE: f64 = 0.000_000_001;

/// Aggregated battle samples used by the DRASTC model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleRecord {
    /// Number of battle samples.
    pub sample_count: u64,
    /// Total battle duration in seconds.
    pub total_duration_seconds: f64,
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
    /// Number of battles with a non-tied lethal casualty outcome.
    pub decisive_battles: u64,
    /// Number of decisive battles won by the perspective side.
    pub wins: u64,
    /// Number of battles with positive kill-point trades.
    pub positive_trades: u64,
}

/// DRASTC evaluator.
#[derive(Debug, Default)]
pub struct DrastcModel {
    aggregate: BattleAggregate,
    theoretical: TheoreticalValues,
    reference_ranges: Option<DrastcReferenceRanges>,
}

impl DrastcModel {
    /// Create an empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add aggregated battle samples to the model.
    pub fn push(&mut self, record: BattleRecord) {
        self.aggregate.push(record);
    }

    /// Set theoretical Rage/Assist values by pairing.
    ///
    /// Unknown Rage pairings default to zero; Assist sums known commander values.
    pub fn set_theoretical(&mut self, primary_commander_id: u32, secondary_commander_id: u32) {
        self.theoretical = theoretical_for_pairing(primary_commander_id, secondary_commander_id);
    }

    /// Return true when the commander pairing exists in the Rage support table.
    pub fn is_supported(primary_commander_id: u32, secondary_commander_id: u32) -> bool {
        is_supported_pairing(primary_commander_id, secondary_commander_id)
    }

    /// Return the number of battle samples in the model.
    pub fn sample_count(&self) -> usize {
        usize::try_from(self.aggregate.sample_count()).unwrap_or(usize::MAX)
    }

    /// Use externally calculated reference ranges for percentile-based scoring.
    pub fn set_reference_ranges(&mut self, reference_ranges: DrastcReferenceRanges) {
        self.reference_ranges = Some(reference_ranges);
    }

    /// Evaluate all records
    pub fn evaluate(&self) -> Option<DrastcScore> {
        if self.aggregate.sample_count() == 0 {
            return None;
        }

        let references = self.reference_ranges?;
        let metrics = self.aggregate.metrics();

        let damage = references.damage.score_curved(metrics.damage_per_second, 0.55);
        let rage = self.theoretical.rage_score();
        let assist = self.theoretical.assist_score();
        let sustainability =
            references.sustainability.score_curved(metrics.sustainability_per_second, 0.55);
        let trade = references.trade.score(metrics.trade_ratio);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kill_points: f64, opponent_kill_points: f64) -> BattleRecord {
        BattleRecord {
            sample_count: 1,
            total_duration_seconds: 100.0,
            kill_points,
            opponent_kill_points,
            opponent_dead: 10.0,
            opponent_severely_wounded: 20.0,
            opponent_slightly_wounded: 70.0,
            sender_dead: 0.0,
            sender_severely_wounded: 10.0,
            sender_slightly_wounded: 30.0,
            sender_healing: 5.0,
            decisive_battles: 1,
            wins: 1,
            positive_trades: u64::from(kill_points > opponent_kill_points),
        }
    }

    fn reference_ranges() -> DrastcReferenceRanges {
        DrastcReferenceRanges {
            damage: ReferenceRange::new(10, 0.0, 4.0),
            sustainability: ReferenceRange::new(10, -2.0, 2.0),
            trade: ReferenceRange::new(10, 0.0, 2.0),
            consistency: ReferenceRange::new(10, 0.0, 1.0),
        }
    }

    fn model_with_references() -> DrastcModel {
        let mut model = DrastcModel::new();
        model.set_reference_ranges(reference_ranges());
        model
    }

    #[test]
    fn evaluate_returns_none_when_no_records_have_been_added() {
        let model = DrastcModel::new();

        assert!(model.evaluate().is_none());
    }

    #[test]
    fn evaluate_returns_none_when_reference_ranges_are_missing() {
        let mut model = DrastcModel::new();
        model.push(record(200.0, 100.0));

        assert!(model.evaluate().is_none());
    }

    #[test]
    fn evaluate_keeps_rage_and_assist_at_zero() {
        let mut model = model_with_references();
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.rage.score, 0.0);
        assert_eq!(score.breakdown.assist.score, 0.0);
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_gang_gamchan_achilles() {
        let mut model = model_with_references();
        model.set_theoretical(579, 575);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 5.47);
        assert_close(score.breakdown.assist.score, 3.39);
    }

    #[test]
    fn is_supported_returns_true_for_pairing_in_rage_table() {
        assert!(DrastcModel::is_supported(579, 575));
    }

    #[test]
    fn is_supported_returns_false_for_pairing_not_in_rage_table() {
        assert!(!DrastcModel::is_supported(575, 540));
    }

    #[test]
    fn is_supported_returns_false_for_unknown_ids() {
        assert!(!DrastcModel::is_supported(1, 2));
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_qin_zhuge_liang() {
        let mut model = model_with_references();
        model.set_theoretical(509, 179);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 9.82);
        assert_close(score.breakdown.assist.score, 5.61);
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_zhuge_liang_prime_hermann() {
        let mut model = model_with_references();
        model.set_theoretical(179, 187);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 6.83);
        assert_close(score.breakdown.assist.score, 8.29);
    }

    #[test]
    fn evaluate_scores_aggregate_metrics() {
        let mut model = model_with_references();
        model.push(BattleRecord {
            sample_count: 3,
            total_duration_seconds: 300.0,
            kill_points: 550.0,
            opponent_kill_points: 300.0,
            opponent_dead: 60.0,
            opponent_severely_wounded: 90.0,
            opponent_slightly_wounded: 300.0,
            sender_dead: 0.0,
            sender_severely_wounded: 25.0,
            sender_slightly_wounded: 55.0,
            sender_healing: 35.0,
            decisive_battles: 3,
            wins: 3,
            positive_trades: 2,
        });

        let score = model.evaluate().expect("score");

        assert_eq!(score.samples, 3);
        assert_close(score.breakdown.damage.value, 1.5);
        assert_close(score.breakdown.sustainability.value, -0.15);
    }

    #[test]
    fn evaluate_curves_damage_and_sustainability_scores() {
        let mut model = model_with_references();
        model.push(BattleRecord {
            sample_count: 1,
            total_duration_seconds: 100.0,
            kill_points: 100.0,
            opponent_kill_points: 100.0,
            opponent_dead: 0.0,
            opponent_severely_wounded: 0.0,
            opponent_slightly_wounded: 300.0,
            sender_dead: 0.0,
            sender_severely_wounded: 0.0,
            sender_slightly_wounded: 0.0,
            sender_healing: 0.0,
            decisive_battles: 0,
            wins: 0,
            positive_trades: 0,
        });

        let score = model.evaluate().expect("score");

        assert!(score.breakdown.damage.score > 5.0);
        assert!(score.breakdown.sustainability.score > 5.0);
    }

    #[test]
    fn evaluate_can_score_against_external_reference_ranges() {
        let reference_ranges = DrastcReferenceRanges {
            damage: ReferenceRange::new(2, 1.2, 2.8),
            sustainability: ReferenceRange::new(2, -1.0, 1.0),
            trade: ReferenceRange::new(2, 0.5, 1.5),
            consistency: ReferenceRange::new(2, 0.0, 1.0),
        };

        let mut model = DrastcModel::new();
        model.set_reference_ranges(reference_ranges);
        model.push(BattleRecord {
            sample_count: 1,
            total_duration_seconds: 100.0,
            kill_points: 100.0,
            opponent_kill_points: 100.0,
            opponent_dead: 0.0,
            opponent_severely_wounded: 0.0,
            opponent_slightly_wounded: 200.0,
            sender_dead: 0.0,
            sender_severely_wounded: 0.0,
            sender_slightly_wounded: 0.0,
            sender_healing: 0.0,
            decisive_battles: 0,
            wins: 0,
            positive_trades: 0,
        });

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.damage.p10, reference_ranges.damage.p10);
        assert_eq!(score.breakdown.damage.p90, reference_ranges.damage.p90);
        assert_eq!(score.breakdown.trade.p10, reference_ranges.trade.p10);
        assert_eq!(score.breakdown.trade.p90, reference_ranges.trade.p90);
        assert!(score.breakdown.damage.score > 5.0);
    }

    #[test]
    fn evaluate_infers_consistency_from_severe_dead_outcome() {
        let mut model = model_with_references();
        model.push(record(50.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.consistency.value, 0.5);
    }

    #[test]
    fn evaluate_scores_equal_trade_ratio_as_five() {
        let mut model = model_with_references();
        model.push(record(100.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.trade.score, 5.0);
    }

    #[test]
    fn evaluate_scores_double_trade_ratio_as_ten() {
        let mut model = model_with_references();
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_eq!(score.breakdown.trade.score, 10.0);
    }

    #[test]
    fn push_adds_aggregate_record() {
        let mut model = model_with_references();
        model.push(BattleRecord { sample_count: 2, ..record(200.0, 100.0) });

        let score = model.evaluate().expect("score");

        assert_eq!(score.samples, 2);
    }

    #[test]
    fn push_combines_aggregate_records() {
        let mut model = model_with_references();
        model.push(record(50.0, 100.0));
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_eq!(model.sample_count(), 2);
        assert_eq!(score.samples, 2);
        assert_close(score.breakdown.trade.value, 1.25);
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 0.02, "actual={actual}, expected={expected}");
    }
}
