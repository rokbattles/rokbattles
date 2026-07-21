use std::collections::BTreeMap;

use rokbattles_drastc::{BattleRecord, DrastcReferenceRanges, ReferenceRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PairingKey {
    pub(super) primary_commander_id: i64,
    pub(super) secondary_commander_id: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct PairingRawTotals {
    pub(super) total_battles: i64,
    pub(super) kill_points_gained: i64,
    pub(super) kill_points_lost: i64,
    pub(super) trade_percentage_total: f64,
    pub(super) battle_duration_total: i64,
    pub(super) severely_wounded_inflicted: i64,
    pub(super) severely_wounded_taken: i64,
    pub(super) damage_total: i64,
    pub(super) sps_total: i64,
    pub(super) tps_total: i64,
    pub(super) healing_total: i64,
    pub(super) opponent_dead: i64,
    pub(super) opponent_slightly_wounded: i64,
    pub(super) sender_dead: i64,
    pub(super) sender_slightly_wounded: i64,
    pub(super) normalized_duration_seconds_total: f64,
    pub(super) decisive_battles: i64,
    pub(super) wins: i64,
    pub(super) positive_trades: i64,
}

impl PairingRawTotals {
    fn accumulate(&mut self, other: Self) {
        self.total_battles += other.total_battles;
        self.kill_points_gained += other.kill_points_gained;
        self.kill_points_lost += other.kill_points_lost;
        self.trade_percentage_total += other.trade_percentage_total;
        self.battle_duration_total += other.battle_duration_total;
        self.severely_wounded_inflicted += other.severely_wounded_inflicted;
        self.severely_wounded_taken += other.severely_wounded_taken;
        self.damage_total += other.damage_total;
        self.sps_total += other.sps_total;
        self.tps_total += other.tps_total;
        self.healing_total += other.healing_total;
        self.opponent_dead += other.opponent_dead;
        self.opponent_slightly_wounded += other.opponent_slightly_wounded;
        self.sender_dead += other.sender_dead;
        self.sender_slightly_wounded += other.sender_slightly_wounded;
        self.normalized_duration_seconds_total += other.normalized_duration_seconds_total;
        self.decisive_battles += other.decisive_battles;
        self.wins += other.wins;
        self.positive_trades += other.positive_trades;
    }

    pub(super) fn to_drastc_record(self) -> BattleRecord {
        BattleRecord {
            sample_count: non_negative_i64_to_u64(self.total_battles),
            total_duration_seconds: self.normalized_duration_seconds_total,
            kill_points: self.kill_points_gained as f64,
            opponent_kill_points: self.kill_points_lost as f64,
            opponent_dead: self.opponent_dead as f64,
            opponent_severely_wounded: self.severely_wounded_inflicted as f64,
            opponent_slightly_wounded: self.opponent_slightly_wounded as f64,
            sender_dead: self.sender_dead as f64,
            sender_severely_wounded: self.severely_wounded_taken as f64,
            sender_slightly_wounded: self.sender_slightly_wounded as f64,
            sender_healing: self.healing_total as f64,
            decisive_battles: non_negative_i64_to_u64(self.decisive_battles),
            wins: non_negative_i64_to_u64(self.wins),
            positive_trades: non_negative_i64_to_u64(self.positive_trades),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Strategy {
    OpenField,
    Swarming,
    Rally,
    Garrison,
}

impl Strategy {
    pub(super) const ALL: [Self; 4] =
        [Self::OpenField, Self::Swarming, Self::Rally, Self::Garrison];

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::OpenField => "open_field",
            Self::Swarming => "swarming",
            Self::Rally => "rally",
            Self::Garrison => "garrison",
        }
    }

    pub(super) fn from_str(value: &str) -> Option<Self> {
        match value {
            "open_field" => Some(Self::OpenField),
            "swarming" => Some(Self::Swarming),
            "rally" => Some(Self::Rally),
            "garrison" => Some(Self::Garrison),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct StrategyRawTotals {
    pub(super) totals: PairingRawTotals,
    pub(super) formations: BTreeMap<i64, i64>,
}

impl StrategyRawTotals {
    pub(super) fn accumulate_strategy(&mut self, strategy: &Self) {
        self.totals.accumulate(strategy.totals);
        for (formation, uses) in &strategy.formations {
            *self.formations.entry(*formation).or_default() += uses;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct PairingStrategies {
    pub(super) values: BTreeMap<Strategy, StrategyRawTotals>,
}

impl PairingStrategies {
    pub(super) fn strategy(&self, strategy: Strategy) -> Option<&StrategyRawTotals> {
        self.values.get(&strategy)
    }

    pub(super) fn all(&self) -> StrategyRawTotals {
        let mut all = StrategyRawTotals::default();
        for strategy in Strategy::ALL {
            if let Some(totals) = self.values.get(&strategy) {
                all.accumulate_strategy(totals);
            }
        }
        all
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct PairingSummary {
    pub(super) total_battles: i64,
    pub(super) kill_points_gained: i64,
    pub(super) kill_points_lost: i64,
    pub(super) avg_trade_percentage: f64,
    pub(super) weighted_trade_percentage: f64,
    pub(super) avg_battle_duration: f64,
    pub(super) total_battle_duration: i64,
    pub(super) severely_wounded_inflicted: i64,
    pub(super) severely_wounded_taken: i64,
    pub(super) dps: f64,
    pub(super) sps: f64,
    pub(super) tps: f64,
    pub(super) hps: f64,
}

#[derive(Debug, PartialEq)]
pub(super) struct PairingsAggregation {
    pub(super) strategies: BTreeMap<PairingKey, PairingStrategies>,
    pub(super) drastc_observed: BTreeMap<PairingKey, PairingRawTotals>,
    pub(super) reference_ranges: DrastcReferenceRanges,
}

fn default_reference_ranges() -> DrastcReferenceRanges {
    DrastcReferenceRanges {
        damage: ReferenceRange::new(0, 0.0, 0.0),
        sustainability: ReferenceRange::new(0, 0.0, 0.0),
        trade: ReferenceRange::new(0, 0.0, 0.0),
        consistency: ReferenceRange::new(0, 0.0, 0.0),
    }
}

fn non_negative_i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

impl Default for PairingsAggregation {
    fn default() -> Self {
        Self {
            strategies: BTreeMap::new(),
            drastc_observed: BTreeMap::new(),
            reference_ranges: default_reference_ranges(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_strategy_accumulator_matches_every_additive_breakdown_total() {
        let mut strategies = PairingStrategies::default();
        for (index, strategy) in Strategy::ALL.into_iter().enumerate() {
            let value = i64::try_from(index + 1).expect("small value");
            strategies.values.insert(
                strategy,
                StrategyRawTotals {
                    totals: PairingRawTotals {
                        total_battles: value,
                        kill_points_gained: value * 10,
                        kill_points_lost: value * 5,
                        trade_percentage_total: value as f64 * 125.0,
                        battle_duration_total: value * 1_000,
                        severely_wounded_inflicted: value * 3,
                        severely_wounded_taken: value * 2,
                        damage_total: value * 4,
                        sps_total: value * 3,
                        tps_total: value * 2,
                        healing_total: value,
                        opponent_dead: value * 5,
                        opponent_slightly_wounded: value * 6,
                        sender_dead: value * 7,
                        sender_slightly_wounded: value * 8,
                        normalized_duration_seconds_total: value as f64 * 1.5,
                        decisive_battles: value * 9,
                        wins: value * 10,
                        positive_trades: value * 11,
                    },
                    formations: BTreeMap::from([(0, value), (value, value * 2)]),
                },
            );
        }

        let all = strategies.all();
        assert_eq!(
            all,
            StrategyRawTotals {
                totals: PairingRawTotals {
                    total_battles: 10,
                    kill_points_gained: 100,
                    kill_points_lost: 50,
                    trade_percentage_total: 1_250.0,
                    battle_duration_total: 10_000,
                    severely_wounded_inflicted: 30,
                    severely_wounded_taken: 20,
                    damage_total: 40,
                    sps_total: 30,
                    tps_total: 20,
                    healing_total: 10,
                    opponent_dead: 50,
                    opponent_slightly_wounded: 60,
                    sender_dead: 70,
                    sender_slightly_wounded: 80,
                    normalized_duration_seconds_total: 15.0,
                    decisive_battles: 90,
                    wins: 100,
                    positive_trades: 110,
                },
                formations: BTreeMap::from([(0, 10), (1, 2), (2, 4), (3, 6), (4, 8)]),
            }
        );
    }
}
