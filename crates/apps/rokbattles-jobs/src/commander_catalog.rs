//! Commander rarity and KvK eligibility from the versioned commander dataset.

use std::collections::BTreeMap;

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
            (rarity_supported && season_supported).then_some(id)
        })
        .collect();
    if ids.is_empty() {
        return Err(JobsError::MissingCombatLabCommanders);
    }
    Ok(ids)
}

#[derive(Deserialize)]
struct CommanderDataset {
    commanders: BTreeMap<i64, CommanderDefinition>,
}

#[derive(Deserialize)]
struct CommanderDefinition {
    rarity: String,
    kvk_limit: u8,
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
        for id in [3, 6, 64, 536, 537, 618, 627] {
            assert!(ids.contains(&id), "eligible commander {id} was excluded");
        }
        for id in [22, 33, 103, 179, 509, 575, 999_999] {
            assert!(!ids.contains(&id), "ineligible commander {id} was included");
        }
    }

    #[test]
    fn commander_dataset_requires_kvk_limits() {
        let result =
            yaml_serde::from_str::<CommanderDataset>("commanders:\n  3:\n    rarity: epic\n");
        assert!(result.is_err());
    }
}
