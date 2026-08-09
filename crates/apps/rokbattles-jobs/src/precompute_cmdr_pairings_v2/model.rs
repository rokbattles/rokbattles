use std::collections::BTreeMap;

use mongodb::bson::Bson;

pub(super) const DAY_MS: i64 = 24 * 60 * 60 * 1_000;
pub(super) const WEDGE_FORMATION_ID: i64 = 2;
pub(super) const WEDGE_FORMATION_II_ID: i64 = 19;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct PairingKey {
    pub(super) primary: i64,
    pub(super) secondary: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct RawTotals {
    pub(super) battles: i64,
    pub(super) kill_points_gained: i64,
    pub(super) kill_points_lost: i64,
    pub(super) severely_wounded_inflicted: i64,
    pub(super) severely_wounded_taken: i64,
    pub(super) battle_duration_ms: i64,
    pub(super) rate_duration_ms: i64,
    pub(super) damage: i64,
    pub(super) healing: i64,
}

impl RawTotals {
    pub(super) fn accumulate(&mut self, other: Self) {
        self.battles += other.battles;
        self.kill_points_gained += other.kill_points_gained;
        self.kill_points_lost += other.kill_points_lost;
        self.severely_wounded_inflicted += other.severely_wounded_inflicted;
        self.severely_wounded_taken += other.severely_wounded_taken;
        self.battle_duration_ms += other.battle_duration_ms;
        self.rate_duration_ms += other.rate_duration_ms;
        self.damage += other.damage;
        self.healing += other.healing;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PerformancePoint {
    pub(super) pairing: PairingKey,
    pub(super) month: i64,
    pub(super) day: i64,
    pub(super) scenario: i64,
    pub(super) totals: RawTotals,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct PairingRoot {
    pub(super) drastc: Option<Bson>,
    pub(super) summaries: BTreeMap<(i64, i64), RawTotals>,
    pub(super) governors: BTreeMap<(i64, i64), i64>,
}

impl PairingRoot {
    pub(super) fn accumulate(&mut self, point: PerformancePoint, cutoffs: &[i64; 4]) {
        for (range, cutoff) in cutoffs.iter().enumerate() {
            if point.day >= *cutoff {
                self.summaries
                    .entry((range as i64, point.scenario))
                    .or_default()
                    .accumulate(point.totals);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct FormationAccumulator {
    pub(super) sample: i64,
    pub(super) counts: BTreeMap<i64, i64>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct InscriptionAccumulator {
    pub(super) special: i64,
    pub(super) rare: i64,
    pub(super) common: i64,
    pub(super) special_common: i64,
    pub(super) rare_common: i64,
    pub(super) common_common: i64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BuffAccumulator {
    pub(super) observations: i64,
    pub(super) total_roll: f64,
    pub(super) max_rolls: i64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ArmamentSlotAccumulator {
    pub(super) sample: i64,
    pub(super) inscriptions: InscriptionAccumulator,
    pub(super) buffs: BTreeMap<i64, BuffAccumulator>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct EquipmentSlotAccumulator {
    pub(super) count: i64,
    pub(super) excluded: i64,
    pub(super) special_talent: i64,
    pub(super) normal: i64,
    pub(super) items: BTreeMap<i64, i64>,
    pub(super) iconic: BTreeMap<i64, i64>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LoadoutBucket {
    pub(super) formation: FormationAccumulator,
    pub(super) armaments: BTreeMap<i64, ArmamentSlotAccumulator>,
    pub(super) equipment: BTreeMap<i64, EquipmentSlotAccumulator>,
    pub(super) accessory_sample: i64,
    pub(super) accessory_pairs: BTreeMap<(i64, i64), i64>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MonthLoadouts {
    pub(super) pairing: PairingKey,
    pub(super) month: i64,
    pub(super) buckets: BTreeMap<(i64, i64), LoadoutBucket>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InscriptionRarity {
    Common,
    Rare,
    Special,
}

pub(super) fn canonical_formation_id(id: i64) -> i64 {
    if id == WEDGE_FORMATION_II_ID { WEDGE_FORMATION_ID } else { id }
}

pub(super) fn range_cutoffs(now_ms: i64) -> [i64; 4] {
    [
        now_ms.saturating_sub(365 * DAY_MS),
        now_ms.saturating_sub(183 * DAY_MS),
        now_ms.saturating_sub(30 * DAY_MS),
        now_ms.saturating_sub(7 * DAY_MS),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wedge_two_is_stored_as_wedge() {
        assert_eq!(canonical_formation_id(WEDGE_FORMATION_II_ID), WEDGE_FORMATION_ID);
    }

    #[test]
    fn root_accumulates_only_matching_time_ranges() {
        let mut root = PairingRoot::default();
        let point = PerformancePoint {
            pairing: PairingKey { primary: 1, secondary: 2 },
            month: 0,
            day: 800,
            scenario: 1,
            totals: RawTotals { battles: 3, ..RawTotals::default() },
        };
        root.accumulate(point, &[700, 750, 800, 900]);

        assert_eq!(root.summaries.get(&(0, 1)).map(|value| value.battles), Some(3));
        assert_eq!(root.summaries.get(&(2, 1)).map(|value| value.battles), Some(3));
        assert!(!root.summaries.contains_key(&(3, 1)));
    }
}
