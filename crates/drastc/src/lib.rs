#![forbid(unsafe_code)]

//! DRASTC scoring model by Davor (TKC) and ROK Battles

use serde::Serialize;

const DAMAGE_WEIGHT: f64 = 0.25;
const RAGE_WEIGHT: f64 = 0.20;
const ASSIST_WEIGHT: f64 = 0.10;
const SUSTAINABILITY_WEIGHT: f64 = 0.20;
const TRADE_WEIGHT: f64 = 0.15;
const CONSISTENCY_WEIGHT: f64 = 0.10;

const P10: f64 = 0.10;
const P90: f64 = 0.90;
const TRADE_RATIO_SCORE_FLOOR: f64 = 0.0;
const TRADE_RATIO_SCORE_CEILING: f64 = 2.0;
const MIN_REFERENCE_RANGE: f64 = 0.000_000_001;

/// Battle sample used by the DRASTC model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleRecord {
    /// Battle duration in seconds. Non-positive or non-finite values are treated as one second.
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

    /// Return the number of battle samples in the model.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Evaluate all records
    pub fn evaluate(&self) -> Option<DrastcScore> {
        if self.samples.is_empty() {
            return None;
        }

        let references = ReferenceMetrics::from_population(&self.samples);
        let metrics = self.aggregate.metrics();

        let damage = references.damage.score(metrics.damage_per_second);
        // Rage will likely use a formula like
        // R = 10 * (((10 - avg_cycle) / 6) ^ 0.55), where avg_cycle is
        let rage = CategoryScore::fixed_zero();
        let assist = CategoryScore::fixed_zero();
        let sustainability = references.sustainability.score(metrics.sustainability_per_second);
        let trade = CategoryScore::fixed_range(
            metrics.trade_ratio,
            TRADE_RATIO_SCORE_FLOOR,
            TRADE_RATIO_SCORE_CEILING,
        );
        let consistency = references.consistency.score(metrics.consistency_rate);

        let overall = (damage.score * DAMAGE_WEIGHT)
            + (rage.score * RAGE_WEIGHT)
            + (assist.score * ASSIST_WEIGHT)
            + (sustainability.score * SUSTAINABILITY_WEIGHT)
            + (trade.score * TRADE_WEIGHT)
            + (consistency.score * CONSISTENCY_WEIGHT);

        Some(DrastcScore {
            samples: self.aggregate.sample_count,
            breakdown: DrastcCategories {
                damage,
                rage,
                assist,
                sustainability,
                trade,
                consistency,
            },
            overall,
        })
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

/// Raw aggregate metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    damage_per_second: f64,
    sustainability_per_second: f64,
    trade_ratio: f64,
    consistency_rate: f64,
}

/// Normalized DRASTC category scores.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrastcCategories {
    /// Damage score.
    pub damage: CategoryScore,
    /// Rage score, currently zero by design.
    pub rage: CategoryScore,
    /// Assist/support score, currently zero by design.
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
    /// P10 reference value from the received battle samples.
    pub p10: f64,
    /// P90 reference value from the received battle samples.
    pub p90: f64,
    /// Normalized score on a 0-10 scale.
    pub score: f64,
}

impl CategoryScore {
    fn fixed_zero() -> Self {
        Self { value: 0.0, p10: 0.0, p90: 0.0, score: 0.0 }
    }

    fn fixed_range(value: f64, p10: f64, p90: f64) -> Self {
        ReferenceRange { count: 1, p10, p90 }.score(value)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BattleAggregate {
    sample_count: u64,
    total_duration_seconds: f64,
    inflicted_casualties: f64,
    received_casualties: f64,
    sender_healing: f64,
    decisive_battles: u64,
    wins: u64,
    kill_points: f64,
    opponent_kill_points: f64,
    positive_trades: u64,
}

impl BattleAggregate {
    fn push(&mut self, record: BattleRecord) {
        self.sample_count += 1;
        self.total_duration_seconds += normalized_duration(record.duration_seconds);
        self.kill_points += finite_non_negative(record.kill_points);
        self.opponent_kill_points += finite_non_negative(record.opponent_kill_points);
        self.inflicted_casualties += casualties(
            record.opponent_dead,
            record.opponent_severely_wounded,
            record.opponent_slightly_wounded,
        );
        self.received_casualties += casualties(
            record.sender_dead,
            record.sender_severely_wounded,
            record.sender_slightly_wounded,
        );
        self.sender_healing += finite_non_negative(record.sender_healing);

        let battle_outcome = battle_outcome(record);
        if let Some(perspective_won) = battle_outcome {
            self.decisive_battles += 1;
            if perspective_won {
                self.wins += 1;
            }
        }

        if is_positive_trade(record.kill_points, record.opponent_kill_points) {
            self.positive_trades += 1;
        }
    }

    fn metrics(&self) -> Metrics {
        let duration = self.total_duration_seconds.max(1.0);
        let win_rate = if self.decisive_battles == 0 {
            0.0
        } else {
            self.wins as f64 / self.decisive_battles as f64
        };
        let positive_trade_rate = if self.sample_count == 0 {
            0.0
        } else {
            self.positive_trades as f64 / self.sample_count as f64
        };
        let consistency_rate = consistency_rate_from_parts(
            (self.decisive_battles > 0).then_some(win_rate),
            (self.sample_count > 0).then_some(positive_trade_rate),
        )
        .unwrap_or(0.0);

        Metrics {
            damage_per_second: self.inflicted_casualties / duration,
            sustainability_per_second: (self.sender_healing - self.received_casualties) / duration,
            trade_ratio: trade_ratio(self.kill_points, self.opponent_kill_points),
            consistency_rate,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RecordMetrics {
    damage_per_second: f64,
    sustainability_per_second: f64,
    consistency_rate: Option<f64>,
}

impl RecordMetrics {
    fn from_record(record: BattleRecord) -> Self {
        let duration = normalized_duration(record.duration_seconds);
        let positive_trade = if is_positive_trade(record.kill_points, record.opponent_kill_points) {
            1.0
        } else {
            0.0
        };
        let inferred_win =
            battle_outcome(record).map(|perspective_won| if perspective_won { 1.0 } else { 0.0 });

        Self {
            damage_per_second: casualties(
                record.opponent_dead,
                record.opponent_severely_wounded,
                record.opponent_slightly_wounded,
            ) / duration,
            sustainability_per_second: (finite_non_negative(record.sender_healing)
                - casualties(
                    record.sender_dead,
                    record.sender_severely_wounded,
                    record.sender_slightly_wounded,
                ))
                / duration,
            consistency_rate: consistency_rate_from_parts(inferred_win, Some(positive_trade)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ReferenceMetrics {
    damage: ReferenceRange,
    sustainability: ReferenceRange,
    consistency: ReferenceRange,
}

impl ReferenceMetrics {
    fn from_population(population: &[RecordMetrics]) -> Self {
        Self {
            damage: ReferenceRange::from_values(
                population.iter().map(|metrics| metrics.damage_per_second),
            ),
            sustainability: ReferenceRange::from_values(
                population.iter().map(|metrics| metrics.sustainability_per_second),
            ),
            consistency: ReferenceRange::from_values(
                population.iter().filter_map(|metrics| metrics.consistency_rate),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ReferenceRange {
    count: usize,
    p10: f64,
    p90: f64,
}

impl ReferenceRange {
    fn from_values(values: impl Iterator<Item = f64>) -> Self {
        let mut values = values.filter(|value| value.is_finite()).collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        Self { count: values.len(), p10: percentile(&values, P10), p90: percentile(&values, P90) }
    }

    fn score(self, value: f64) -> CategoryScore {
        let score = if self.count == 0 || !value.is_finite() {
            0.0
        } else if (self.p90 - self.p10).abs() <= MIN_REFERENCE_RANGE {
            5.0
        } else {
            (10.0 * ((value - self.p10) / (self.p90 - self.p10))).clamp(0.0, 10.0)
        };

        CategoryScore { value, p10: self.p10, p90: self.p90, score }
    }
}

fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
    match sorted_values {
        [] => 0.0,
        [value] => *value,
        values => {
            let rank = percentile.clamp(0.0, 1.0) * (values.len() - 1) as f64;
            let lower = rank.floor() as usize;
            let upper = rank.ceil() as usize;
            let lower_value = values.get(lower).copied().unwrap_or(0.0);
            let upper_value = values.get(upper).copied().unwrap_or(lower_value);
            lower_value + ((upper_value - lower_value) * (rank - lower as f64))
        }
    }
}

fn normalized_duration(duration_seconds: f64) -> f64 {
    if duration_seconds.is_finite() && duration_seconds > 0.0 { duration_seconds } else { 1.0 }
}

fn casualties(dead: f64, severely_wounded: f64, slightly_wounded: f64) -> f64 {
    finite_non_negative(dead)
        + finite_non_negative(severely_wounded)
        + finite_non_negative(slightly_wounded)
}

fn lethal_casualties(dead: f64, severely_wounded: f64) -> f64 {
    finite_non_negative(dead) + finite_non_negative(severely_wounded)
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 { value } else { 0.0 }
}

fn is_positive_trade(kill_points: f64, opponent_kill_points: f64) -> bool {
    finite_non_negative(kill_points) > finite_non_negative(opponent_kill_points)
}

fn battle_outcome(record: BattleRecord) -> Option<bool> {
    let inflicted = lethal_casualties(record.opponent_dead, record.opponent_severely_wounded);
    let received = lethal_casualties(record.sender_dead, record.sender_severely_wounded);
    if (inflicted - received).abs() <= MIN_REFERENCE_RANGE {
        None
    } else {
        Some(inflicted > received)
    }
}

fn consistency_rate_from_parts(
    win_rate: Option<f64>,
    positive_trade_rate: Option<f64>,
) -> Option<f64> {
    match (win_rate, positive_trade_rate) {
        (Some(win_rate), Some(positive_trade_rate)) => Some((win_rate + positive_trade_rate) / 2.0),
        (Some(win_rate), None) => Some(win_rate),
        (None, Some(positive_trade_rate)) => Some(positive_trade_rate),
        (None, None) => None,
    }
}

fn trade_ratio(kill_points: f64, opponent_kill_points: f64) -> f64 {
    let kill_points = finite_non_negative(kill_points);
    let opponent_kill_points = finite_non_negative(opponent_kill_points);
    if kill_points <= MIN_REFERENCE_RANGE && opponent_kill_points <= MIN_REFERENCE_RANGE {
        1.0
    } else if opponent_kill_points <= MIN_REFERENCE_RANGE {
        0.0
    } else {
        kill_points / opponent_kill_points
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

    #[test]
    fn percentile_interpolates_sorted_values() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];

        assert_eq!(percentile(&values, 0.10), 1.4);
        assert_eq!(percentile(&values, 0.90), 4.6);
    }
}
