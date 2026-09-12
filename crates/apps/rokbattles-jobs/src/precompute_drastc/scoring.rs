use std::collections::BTreeMap;

use rokbattles_drastc::{DrastcModel, DrastcReferenceRanges, DrastcScore, RageTable};

use super::model::{PairingKey, PairingRawTotals};

pub(super) fn build_drastc_scores_from_aggregates(
    observed: &BTreeMap<PairingKey, PairingRawTotals>,
    supported_pairings: &[PairingKey],
    reference_ranges: DrastcReferenceRanges,
    rage_table: RageTable,
) -> BTreeMap<PairingKey, DrastcScore> {
    let mut scores = BTreeMap::new();

    for key in supported_pairings {
        let Some(raw) = observed.get(key) else {
            continue;
        };
        let (Ok(primary), Ok(secondary)) =
            (u32::try_from(key.primary_commander_id), u32::try_from(key.secondary_commander_id))
        else {
            continue;
        };

        let mut model = DrastcModel::new();
        model.set_rage_table(rage_table);
        model.set_reference_ranges(reference_ranges);
        model.set_theoretical(primary, secondary);
        model.push(raw.to_drastc_record());

        if let Some(score) = model.evaluate() {
            scores.insert(*key, score);
        }
    }

    scores
}

pub(super) fn supported_drastc_pairings(
    commander_ids: &[i64],
    rage_table: RageTable,
) -> Vec<PairingKey> {
    ordered_pairing_keys(commander_ids)
        .filter(|key| {
            u32::try_from(key.primary_commander_id)
                .ok()
                .zip(u32::try_from(key.secondary_commander_id).ok())
                .is_some_and(|(primary, secondary)| {
                    DrastcModel::is_supported(rage_table, primary, secondary)
                })
        })
        .collect()
}

fn ordered_pairing_keys(commander_ids: &[i64]) -> impl Iterator<Item = PairingKey> + '_ {
    commander_ids.iter().flat_map(|primary| {
        commander_ids.iter().filter(move |secondary| primary != *secondary).map(move |secondary| {
            PairingKey { primary_commander_id: *primary, secondary_commander_id: *secondary }
        })
    })
}

#[cfg(test)]
mod tests {
    use rokbattles_drastc::{
        DrastcReferenceRanges, PRESOC_RAGE_TABLE, ReferenceRange, SOC_RAGE_TABLE,
    };

    use super::*;

    #[test]
    fn supported_drastc_pairings_returns_only_model_supported_pairings() {
        let keys = supported_drastc_pairings(&[575, 579, 540], SOC_RAGE_TABLE);

        assert!(
            keys.contains(&PairingKey { primary_commander_id: 579, secondary_commander_id: 575 })
        );
        assert!(
            !keys.contains(&PairingKey { primary_commander_id: 575, secondary_commander_id: 540 })
        );
    }

    #[test]
    fn scores_should_skip_commander_ids_that_would_wrap_to_supported_ids() {
        let wrap = i64::from(u32::MAX) + 1;
        let keys = [
            PairingKey { primary_commander_id: 579 - wrap, secondary_commander_id: 575 },
            PairingKey { primary_commander_id: 579, secondary_commander_id: 575 + wrap },
        ];
        let observed = keys
            .iter()
            .map(|key| (*key, PairingRawTotals { total_battles: 2, ..Default::default() }))
            .collect();
        let ranges = DrastcReferenceRanges {
            damage: ReferenceRange::new(10, 0.0, 4.0),
            sustainability: ReferenceRange::new(10, -2.0, 2.0),
            trade: ReferenceRange::new(10, 0.0, 2.0),
            consistency: ReferenceRange::new(10, 0.0, 1.0),
        };

        let scores = build_drastc_scores_from_aggregates(&observed, &keys, ranges, SOC_RAGE_TABLE);

        assert!(scores.is_empty());
    }

    #[test]
    fn build_drastc_scores_from_aggregates_scores_supported_observed_pairings() {
        let key = PairingKey { primary_commander_id: 579, secondary_commander_id: 575 };
        let observed = BTreeMap::from([(
            key,
            PairingRawTotals {
                total_battles: 2,
                kill_points_gained: 250,
                kill_points_lost: 200,
                severely_wounded_inflicted: 25,
                severely_wounded_taken: 25,
                healing_total: 15,
                opponent_dead: 10,
                opponent_slightly_wounded: 90,
                sender_dead: 10,
                sender_slightly_wounded: 70,
                normalized_duration_seconds_total: 150.0,
                decisive_battles: 2,
                wins: 1,
                positive_trades: 1,
            },
        )]);
        let ranges = DrastcReferenceRanges {
            damage: ReferenceRange::new(10, 0.0, 4.0),
            sustainability: ReferenceRange::new(10, -2.0, 2.0),
            trade: ReferenceRange::new(10, 0.0, 2.0),
            consistency: ReferenceRange::new(10, 0.0, 1.0),
        };

        let scores = build_drastc_scores_from_aggregates(&observed, &[key], ranges, SOC_RAGE_TABLE);

        let score = scores.get(&key).expect("drastc score");
        assert_eq!(score.samples, 2);
        assert_eq!(score.breakdown.rage.value, 8.0);
        assert_eq!(score.breakdown.assist.value, 14.24);
    }

    #[test]
    fn presoc_table_pairings_are_eligible_and_use_presoc_rage_values() {
        use crate::{
            combat_lab_season::CombatLabSeason, commander_catalog::combat_lab_commander_ids,
        };
        let ids = combat_lab_commander_ids(CombatLabSeason::PreSoc).expect("pre-SoC commanders");
        let keys = supported_drastc_pairings(&ids, PRESOC_RAGE_TABLE);
        assert_eq!(keys.len(), PRESOC_RAGE_TABLE.len());
        let key = PairingKey { primary_commander_id: 3, secondary_commander_id: 6 };
        assert!(keys.contains(&key));
        assert!(!supported_drastc_pairings(&ids, SOC_RAGE_TABLE).contains(&key));
        let observed = BTreeMap::from([(
            key,
            PairingRawTotals {
                total_battles: 10,
                kill_points_gained: 100,
                kill_points_lost: 50,
                normalized_duration_seconds_total: 100.0,
                ..Default::default()
            },
        )]);
        let scores = build_drastc_scores_from_aggregates(
            &observed,
            &keys,
            DrastcReferenceRanges {
                damage: ReferenceRange::new(10, 0.0, 4.0),
                sustainability: ReferenceRange::new(10, -2.0, 2.0),
                trade: ReferenceRange::new(10, 0.0, 2.0),
                consistency: ReferenceRange::new(10, 0.0, 1.0),
            },
            PRESOC_RAGE_TABLE,
        );
        assert_eq!(scores.get(&key).expect("Sun Tzu/YSG score").breakdown.rage.value, 7.5);
    }
}
