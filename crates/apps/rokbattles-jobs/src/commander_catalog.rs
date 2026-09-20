//! Commander rarity and KvK eligibility from the versioned commander dataset.

use std::collections::BTreeMap;

use mongodb::bson::{Document, doc};
use serde::Deserialize;

use crate::{combat_lab_season::CombatLabSeason, error::JobsError};

const COMMANDERS_YAML: &str = include_str!("../../../../datasets/commanders.yaml");

pub(crate) fn combat_lab_commander_ids(season: CombatLabSeason) -> Result<Vec<i64>, JobsError> {
    let dataset: CommanderDataset = yaml_serde::from_str(COMMANDERS_YAML)?;
    let ids: Vec<_> = dataset
        .commanders
        .into_iter()
        .filter_map(|(id, commander)| {
            let rarity_supported = matches!(commander.rarity.as_str(), "legendary" | "epic");
            let season_supported = season == CombatLabSeason::Soc || commander.kvk_limit < 3;
            let gathering = commander.talents.iter().any(|talent| talent == "gathering");
            (rarity_supported && season_supported && !gathering).then_some(id)
        })
        .collect();
    if ids.is_empty() {
        return Err(JobsError::MissingCombatLabCommanders);
    }
    Ok(ids)
}

/// Apply after unwinding opponents so an excluded march removes only its own fight.
pub(crate) fn eligible_battle_match(commander_ids: &[i64]) -> Document {
    doc! { "$match": {
        "sender.commanders.primary.id": { "$in": commander_ids },
        "sender.commanders.secondary.id": { "$in": commander_ids },
        "opponents.commanders.primary.id": { "$in": commander_ids },
        "opponents.commanders.secondary.id": { "$in": commander_ids },
    } }
}

#[derive(Deserialize)]
struct CommanderDataset {
    commanders: BTreeMap<i64, CommanderDefinition>,
}

#[derive(Deserialize)]
struct CommanderDefinition {
    rarity: String,
    kvk_limit: u8,
    talents: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_lab_commander_ids_reads_expected_dataset_values() {
        let ids = combat_lab_commander_ids(CombatLabSeason::Soc).expect("Combat Lab IDs");

        assert!(ids.contains(&509));
        assert!(ids.contains(&6));
        assert!(ids.contains(&179));
        assert!(ids.contains(&187));
        assert!(ids.contains(&3));
        assert!(ids.contains(&12));
        assert!(ids.contains(&537));
        assert!(!ids.contains(&22));
        assert!(!ids.contains(&33));
    }

    #[test]
    fn presoc_includes_epic_and_legendary_commanders_below_kvk_limit_three() {
        let ids = combat_lab_commander_ids(CombatLabSeason::PreSoc).expect("pre-SoC IDs");
        for id in [3, 6, 64, 536, 537, 618] {
            assert!(ids.contains(&id), "eligible commander {id} was excluded");
        }
        for id in [22, 33, 103, 179, 509, 575, 627, 999_999] {
            assert!(!ids.contains(&id), "ineligible commander {id} was included");
        }
    }

    #[test]
    fn combat_lab_excludes_gatherers_and_low_rarities_in_both_seasons() {
        for season in [CombatLabSeason::Soc, CombatLabSeason::PreSoc] {
            let ids = combat_lab_commander_ids(season).expect("eligible commanders");
            for id in [15, 24, 32, 33, 34, 38, 102, 105, 116, 134, 463, 572, 573, 627, 22] {
                assert!(!ids.contains(&id), "excluded commander {id} was included in {season:?}");
            }
            assert!(ids.contains(&3), "epic combat commander must remain eligible");
        }
    }

    #[test]
    fn commander_dataset_requires_kvk_limits() {
        let result = yaml_serde::from_str::<CommanderDataset>(
            "commanders:\n  3:\n    rarity: epic\n    talents: []\n",
        );
        assert!(result.is_err());
    }

    #[test]
    fn fight_filter_requires_eligible_primary_and_secondary_on_each_side() {
        let stage = eligible_battle_match(&[3, 6]);
        let matcher = stage.get_document("$match").expect("battle match");
        for field in [
            "sender.commanders.primary.id",
            "sender.commanders.secondary.id",
            "opponents.commanders.primary.id",
            "opponents.commanders.secondary.id",
        ] {
            assert_eq!(matcher.get_document(field), Ok(&doc! { "$in": [3_i64, 6_i64] }));
        }
    }
}
