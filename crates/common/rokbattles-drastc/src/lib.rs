#![forbid(unsafe_code)]

//! Scores commander pairings from aggregated battle data using DRASTC.
//!
//! DRASTC combines Damage, Rage, Assist, Sustainability, Trade, and Consistency.
//! The scoring model is by Davor (TKC) and ROK Battles.
//!
//! Add [`BattleRecord`] totals to a [`DrastcModel`], supply externally calculated
//! [`DrastcReferenceRanges`], and call [`DrastcModel::evaluate`]. Select a
//! [`RageTable`] and commander pairing to include theoretical Rage and Assist.
//! The caller chooses which battles belong in the sample and reference population;
//! this crate does not load reports, filter battles, or calculate percentiles.
//!
//! # Scoring
//!
//! Records are summed before calculating rates. Casualties include dead,
//! severely wounded, and slightly wounded units with equal weight. Per-second
//! metrics use the combined duration, floored at one second.
//!
//! | Category | Input | Overall weight |
//! | --- | --- | --- |
//! | Damage | Inflicted casualties per second | 25% |
//! | Rage | The ordered pairing's theoretical average skill cycle | 15% |
//! | Assist | Sum of the two commanders' static support values | 10% |
//! | Sustainability | Healing minus received casualties, per second | 20% |
//! | Trade | Sender kill points divided by opponent kill points | 20% |
//! | Consistency | Mean of the available win and positive-trade rates | 10% |
//!
//! Damage, Sustainability, Trade, and Consistency use the supplied P10/P90
//! bounds. Linear scores are clamped to `0..=10`; Damage and Sustainability
//! then apply `10 * (score / 10)^0.55`. Rage and Assist use fixed bounds and
//! the same exponent. The overall score sums the six weighted scores, keeping
//! the weights of categories that score zero.
//!
//! Trade has two special cases at a tolerance of `1e-9`: if both kill-point
//! totals are at or below the tolerance, the ratio is 1; if only the opponent's
//! total is, the ratio is 0. Consistency uses wins divided by decisive battles
//! and positive trades divided by all samples, omitting a rate with no samples.
//!
//! [`DrastcConfidence`] separately describes sample size and governor
//! concentration. It does not change the performance score.
//!
//! # Examples
//!
//! Configure an evaluation with illustrative reference bounds:
//!
//! ```
//! use rokbattles_drastc::{
//!     BattleRecord, DrastcModel, DrastcReferenceRanges, ReferenceRange, SOC_RAGE_TABLE,
//! };
//!
//! let mut model = DrastcModel::new();
//! model.set_rage_table(SOC_RAGE_TABLE);
//! model.set_theoretical(579, 575);
//! model.set_reference_ranges(DrastcReferenceRanges {
//!     damage: ReferenceRange::new(100, 0.0, 4.0),
//!     sustainability: ReferenceRange::new(100, -2.0, 2.0),
//!     trade: ReferenceRange::new(100, 0.0, 2.0),
//!     consistency: ReferenceRange::new(100, 0.0, 1.0),
//! });
//! model.push(BattleRecord {
//!     sample_count: 1,
//!     total_duration_seconds: 100.0,
//!     kill_points: 200.0,
//!     opponent_kill_points: 100.0,
//!     opponent_dead: 10.0,
//!     opponent_severely_wounded: 20.0,
//!     opponent_slightly_wounded: 70.0,
//!     sender_dead: 0.0,
//!     sender_severely_wounded: 10.0,
//!     sender_slightly_wounded: 30.0,
//!     sender_healing: 5.0,
//!     decisive_battles: 1,
//!     wins: 1,
//!     positive_trades: 1,
//! });
//!
//! let score = model.evaluate().expect("samples and references are present");
//! assert_eq!(score.samples, 1);
//! assert_eq!(score.breakdown.damage.value, 1.0);
//! ```

mod aggregate;
mod confidence;
mod metrics;
mod reference;
mod theoretical;
mod weights;

use aggregate::BattleAggregate;
pub use confidence::DrastcConfidence;
pub use reference::{DrastcReferenceRanges, ReferenceRange};
use serde::Serialize;
pub use theoretical::{
    PRESOC_RAGE_TABLE, RagePairing, RageTable, SOC_RAGE_TABLE, TheoreticalValues,
};
use theoretical::{is_supported_pairing, theoretical_for_pairing};
use weights::weighted_overall;

pub(crate) const MIN_REFERENCE_RANGE: f64 = 0.000_000_001;

/// Totals for one or more battles from the sender's perspective.
///
/// Supply totals rather than per-battle averages. `sender_*` fields describe
/// the side being scored; `opponent_*` fields describe the other side. The
/// caller classifies decisive battles, wins, and positive trades before pushing
/// a record; the model does not infer them from casualty or kill-point totals.
///
/// On ingestion, negative and non-finite floating-point fields contribute zero.
/// Wins are capped at `decisive_battles`, and positive trades at `sample_count`,
/// separately for each record. Other relationships between fields are not
/// validated. Keep cumulative integer counts within `u64` and floating-point
/// totals finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleRecord {
    /// Number of battles represented by these totals.
    pub sample_count: u64,
    /// Total battle duration in seconds.
    pub total_duration_seconds: f64,
    /// Total kill points earned by the sender.
    pub kill_points: f64,
    /// Total kill points earned by the opponent.
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
    /// Total units healed by the sender.
    pub sender_healing: f64,
    /// Number of battles with a non-tied lethal casualty outcome.
    pub decisive_battles: u64,
    /// Number of decisive battles won by the perspective side.
    pub wins: u64,
    /// Number of battles in which the sender earned more kill points than the opponent.
    pub positive_trades: u64,
}

/// Accumulates battle totals and evaluates a commander pairing.
///
/// An empty model has no samples, reference ranges, or commander pairing, and
/// uses an empty Rage table. See the [crate example](crate#examples) for setup.
/// Evaluation reads the accumulated totals without consuming or resetting them.
#[derive(Debug, Default)]
pub struct DrastcModel {
    aggregate: BattleAggregate,
    rage_table: RageTable,
    commander_pairing: Option<(u32, u32)>,
    reference_ranges: Option<DrastcReferenceRanges>,
}

impl DrastcModel {
    /// Creates an empty model with the same settings as [`Default::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a record's totals to the model using the rules on [`BattleRecord`].
    ///
    /// Records are not retained or deduplicated. A record with zero samples still
    /// contributes its other fields, so its totals should also be zero.
    pub fn push(&mut self, record: BattleRecord) {
        self.aggregate.push(record);
    }

    /// Selects the Rage table used by subsequent evaluations.
    ///
    /// The stored pairing is looked up when [`evaluate`](Self::evaluate) runs,
    /// so this can be called before or after [`set_theoretical`](Self::set_theoretical).
    /// Assist values are independent of this table.
    pub fn set_rage_table(&mut self, rage_table: RageTable) {
        self.rage_table = rage_table;
    }

    /// Selects the ordered commander pairing used for theoretical Rage and Assist.
    ///
    /// A pairing absent from the selected Rage table has a zero Rage score.
    /// Assist sums the known commander values, treating unknown IDs as zero.
    /// This stores the IDs without checking support; use [`is_supported`](Self::is_supported)
    /// if the caller needs to reject an unlisted pairing.
    pub fn set_theoretical(&mut self, primary_commander_id: u32, secondary_commander_id: u32) {
        self.commander_pairing = Some((primary_commander_id, secondary_commander_id));
    }

    /// Returns whether the exact ordered pairing appears in `rage_table`.
    ///
    /// Swapping the IDs can change the result. This checks table membership,
    /// not whether the entry's cycle value is valid or has Assist data.
    pub fn is_supported(
        rage_table: RageTable,
        primary_commander_id: u32,
        secondary_commander_id: u32,
    ) -> bool {
        is_supported_pairing(rage_table, primary_commander_id, secondary_commander_id)
    }

    /// Returns the sum of the records' sample counts, capped at `usize::MAX`.
    ///
    /// This counts represented battles, not calls to [`push`](Self::push).
    pub fn sample_count(&self) -> usize {
        usize::try_from(self.aggregate.sample_count()).unwrap_or(usize::MAX)
    }

    /// Sets the precomputed bounds for the four battle-derived categories.
    ///
    /// Values are stored without validation. See [`ReferenceRange::new`] for
    /// the expected inputs and the behavior of empty or collapsed ranges.
    pub fn set_reference_ranges(&mut self, reference_ranges: DrastcReferenceRanges) {
        self.reference_ranges = Some(reference_ranges);
    }

    /// Scores the accumulated totals using the current references and pairing.
    ///
    /// Returns `None` if the total sample count is zero or reference ranges have
    /// not been set. Individual ranges with zero samples still permit evaluation
    /// but score their category at zero. Without a selected pairing, Rage and
    /// Assist are zero; an unlisted pairing only forces Rage to zero.
    ///
    /// See the [scoring formulas](crate#scoring) for category inputs and weights.
    pub fn evaluate(&self) -> Option<DrastcScore> {
        if self.aggregate.sample_count() == 0 {
            return None;
        }

        let references = self.reference_ranges?;
        let metrics = self.aggregate.metrics();
        let theoretical =
            self.commander_pairing.map_or_else(TheoreticalValues::default, |pairing| {
                theoretical_for_pairing(self.rage_table, pairing.0, pairing.1)
            });

        let damage = references.damage.score_curved(metrics.damage_per_second, 0.55);
        let rage = theoretical.rage_score();
        let assist = theoretical.assist_score();
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

/// A DRASTC evaluation with its sample count and category breakdown.
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
    /// Inflicted casualties per second, scored against the Damage reference range.
    pub damage: CategoryScore,
    /// Theoretical skill cycle, scored with shorter cycles receiving higher scores.
    pub rage: CategoryScore,
    /// Combined static support value, scored against fixed bounds of 0 and 100.
    pub assist: CategoryScore,
    /// Healing minus received casualties per second; the raw value can be negative.
    pub sustainability: CategoryScore,
    /// Aggregate kill-point ratio, including the zero-denominator rules in the crate docs.
    pub trade: CategoryScore,
    /// Mean of the available win and positive-trade rates.
    pub consistency: CategoryScore,
}

/// One category's metric value, reference bounds, and normalized score.
///
/// Rage uses reversed bounds (10 and 4) because shorter cycles score higher.
/// Missing or invalid theoretical inputs produce all-zero fields. For the
/// battle-derived categories, the original metric and bounds are retained even
/// when the resulting score is zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScore {
    /// Metric before normalization, in the units described by [`DrastcCategories`].
    pub value: f64,
    /// Lower-scoring reference bound: supplied P10, or the category's fixed bound.
    pub p10: f64,
    /// Higher-scoring reference bound: supplied P90, or the category's fixed bound.
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
        model.set_rage_table(SOC_RAGE_TABLE);
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

        assert_eq!(score.breakdown.rage.score.to_bits(), 0.0_f64.to_bits());
        assert_eq!(score.breakdown.assist.score.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_gang_gamchan_achilles() {
        let mut model = model_with_references();
        model.set_theoretical(579, 575);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 5.47);
        assert_close(score.breakdown.assist.score, 3.42);
    }

    #[test]
    fn is_supported_returns_true_for_pairing_in_rage_table() {
        assert!(DrastcModel::is_supported(SOC_RAGE_TABLE, 579, 575));
    }

    #[test]
    fn is_supported_returns_false_for_pairing_not_in_rage_table() {
        assert!(!DrastcModel::is_supported(SOC_RAGE_TABLE, 575, 540));
    }

    #[test]
    fn is_supported_returns_false_for_unknown_ids() {
        assert!(!DrastcModel::is_supported(SOC_RAGE_TABLE, 1, 2));
    }

    #[test]
    fn is_supported_uses_the_explicit_rage_table() {
        assert!(DrastcModel::is_supported(PRESOC_RAGE_TABLE, 141, 6));
        assert!(!DrastcModel::is_supported(SOC_RAGE_TABLE, 141, 6));
    }

    #[test]
    fn presoc_rage_uses_epic_sun_tzu_and_legendary_pelagius() {
        assert!(DrastcModel::is_supported(PRESOC_RAGE_TABLE, 3, 99));
        assert!(DrastcModel::is_supported(PRESOC_RAGE_TABLE, 9, 618));
        assert!(!DrastcModel::is_supported(PRESOC_RAGE_TABLE, 595, 99));
        assert!(!DrastcModel::is_supported(PRESOC_RAGE_TABLE, 9, 18));
    }

    #[test]
    fn evaluate_uses_selected_presoc_rage_table() {
        let mut model = model_with_references();
        model.set_theoretical(141, 6);
        model.set_rage_table(PRESOC_RAGE_TABLE);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert!((score.breakdown.rage.value - 8.5).abs() < 1e-12);
        assert!((score.breakdown.assist.value - 0.73125).abs() < 1e-12);
    }

    #[test]
    fn evaluate_accepts_a_custom_static_rage_table() {
        const CUSTOM_RAGE_TABLE: RageTable = &[RagePairing::new(1, 2, 6.0)];
        let mut model = model_with_references();
        model.set_rage_table(CUSTOM_RAGE_TABLE);
        model.set_theoretical(1, 2);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert!((score.breakdown.rage.value - 6.0).abs() < 1e-12);
    }

    #[test]
    fn evaluate_uses_known_theoretical_values_for_qin_zhuge_liang() {
        let mut model = model_with_references();
        model.set_theoretical(509, 179);
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert_close(score.breakdown.rage.score, 9.82);
        assert_close(score.breakdown.assist.score, 5.57);
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

        assert_eq!(score.breakdown.damage.p10.to_bits(), reference_ranges.damage.p10.to_bits());
        assert_eq!(score.breakdown.damage.p90.to_bits(), reference_ranges.damage.p90.to_bits());
        assert_eq!(score.breakdown.trade.p10.to_bits(), reference_ranges.trade.p10.to_bits());
        assert_eq!(score.breakdown.trade.p90.to_bits(), reference_ranges.trade.p90.to_bits());
        assert!(score.breakdown.damage.score > 5.0);
    }

    #[test]
    fn evaluate_infers_consistency_from_severe_dead_outcome() {
        let mut model = model_with_references();
        model.push(record(50.0, 100.0));

        let score = model.evaluate().expect("score");

        assert!((score.breakdown.consistency.value - 0.5).abs() < 1e-12);
    }

    #[test]
    fn evaluate_scores_equal_trade_ratio_as_five() {
        let mut model = model_with_references();
        model.push(record(100.0, 100.0));

        let score = model.evaluate().expect("score");

        assert!((score.breakdown.trade.score - 5.0).abs() < 1e-12);
    }

    #[test]
    fn evaluate_scores_double_trade_ratio_as_ten() {
        let mut model = model_with_references();
        model.push(record(200.0, 100.0));

        let score = model.evaluate().expect("score");

        assert!((score.breakdown.trade.score - 10.0).abs() < 1e-12);
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
