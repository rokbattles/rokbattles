use std::collections::BTreeMap;

use rokbattles_drastc::{DrastcModel, DrastcReferenceRanges, DrastcScore, SOC_RAGE_TABLE};

use super::model::{PairingKey, PairingRawTotals};

pub(super) fn build_drastc_scores_from_aggregates(
    observed: &BTreeMap<PairingKey, PairingRawTotals>,
    supported_pairings: &[PairingKey],
    reference_ranges: DrastcReferenceRanges,
) -> BTreeMap<PairingKey, DrastcScore> {
    let mut scores = BTreeMap::new();

    for key in supported_pairings {
        let Some(raw) = observed.get(key) else {
            continue;
        };

        let mut model = DrastcModel::new();
        model.set_rage_table(SOC_RAGE_TABLE);
        model.set_reference_ranges(reference_ranges);
        model.set_theoretical(key.primary_commander_id as u32, key.secondary_commander_id as u32);
        model.push(raw.to_drastc_record());

        if let Some(score) = model.evaluate() {
            scores.insert(*key, score);
        }
    }

    scores
}

pub(super) fn supported_drastc_pairings(legendary_ids: &[i64]) -> Vec<PairingKey> {
    ordered_pairing_keys(legendary_ids)
        .filter(|key| {
            u32::try_from(key.primary_commander_id)
                .ok()
                .zip(u32::try_from(key.secondary_commander_id).ok())
                .is_some_and(|(primary, secondary)| {
                    DrastcModel::is_supported(SOC_RAGE_TABLE, primary, secondary)
                })
        })
        .collect()
}

fn ordered_pairing_keys(legendary_ids: &[i64]) -> impl Iterator<Item = PairingKey> + '_ {
    legendary_ids.iter().flat_map(|primary| {
        legendary_ids.iter().filter(move |secondary| primary != *secondary).map(move |secondary| {
            PairingKey { primary_commander_id: *primary, secondary_commander_id: *secondary }
        })
    })
}

#[cfg(test)]
mod tests {
    use rokbattles_drastc::{DrastcReferenceRanges, ReferenceRange};

    use super::*;

    #[test]
    fn supported_drastc_pairings_returns_only_model_supported_pairings() {
        let keys = supported_drastc_pairings(&[575, 579, 540]);

        assert!(
            keys.contains(&PairingKey { primary_commander_id: 579, secondary_commander_id: 575 })
        );
        assert!(
            !keys.contains(&PairingKey { primary_commander_id: 575, secondary_commander_id: 540 })
        );
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

        let scores = build_drastc_scores_from_aggregates(&observed, &[key], ranges);

        let score = scores.get(&key).expect("drastc score");
        assert_eq!(score.samples, 2);
        assert_eq!(score.breakdown.rage.value, 8.0);
        assert_eq!(score.breakdown.assist.value, 14.24);
    }
}
