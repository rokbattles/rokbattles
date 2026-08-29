use std::collections::BTreeMap;

use rokbattles_drastc::{BattleRecord, DrastcReferenceRanges, ReferenceRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PairingKey {
    pub(super) primary_commander_id: i64,
    pub(super) secondary_commander_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Strategy {
    OpenField,
    Swarming,
    Rally,
    Garrison,
}

impl Strategy {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::OpenField => "open_field",
            Self::Swarming => "swarming",
            Self::Rally => "rally",
            Self::Garrison => "garrison",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct PairingRawTotals {
    pub(super) total_battles: i64,
    pub(super) kill_points_gained: i64,
    pub(super) kill_points_lost: i64,
    pub(super) severely_wounded_inflicted: i64,
    pub(super) severely_wounded_taken: i64,
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
    pub(super) fn to_drastc_record(self) -> BattleRecord {
        BattleRecord {
            sample_count: to_u64(self.total_battles),
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
            decisive_battles: to_u64(self.decisive_battles),
            wins: to_u64(self.wins),
            positive_trades: to_u64(self.positive_trades),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct DrastcAggregation {
    pub(super) observed: BTreeMap<PairingKey, PairingRawTotals>,
    pub(super) reference_ranges: DrastcReferenceRanges,
}

impl Default for DrastcAggregation {
    fn default() -> Self {
        Self {
            observed: BTreeMap::new(),
            reference_ranges: DrastcReferenceRanges {
                damage: ReferenceRange::new(0, 0.0, 0.0),
                sustainability: ReferenceRange::new(0, 0.0, 0.0),
                trade: ReferenceRange::new(0, 0.0, 0.0),
                consistency: ReferenceRange::new(0, 0.0, 0.0),
            },
        }
    }
}

fn to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}
