use std::collections::HashMap;

use serde::Deserialize;

use super::model::InscriptionRarity;
use crate::error::JobsError;

const INSCRIPTIONS_YAML: &str = include_str!("../../../../../datasets/inscriptions.yaml");
const ARMAMENTS_YAML: &str = include_str!("../../../../../datasets/armaments.yaml");
const EQUIPMENT_YAML: &str = include_str!("../../../../../datasets/equipment.yaml");

#[derive(Debug)]
pub(super) struct Catalogs {
    pub(super) inscriptions: HashMap<i64, InscriptionRarity>,
    pub(super) armament_max_rolls: HashMap<i64, f64>,
    pub(super) equipment_qualities: HashMap<i64, i64>,
}

impl Catalogs {
    pub(super) fn load() -> Result<Self, JobsError> {
        let inscriptions = read_inscriptions()?;
        let armament_max_rolls = read_armament_rolls()?;
        let equipment_qualities = read_equipment_qualities()?;
        if inscriptions.is_empty()
            || armament_max_rolls.is_empty()
            || equipment_qualities.is_empty()
        {
            return Err(JobsError::InvalidCombatLabData(
                "one or more embedded game catalogs are empty".to_owned(),
            ));
        }

        Ok(Self { inscriptions, armament_max_rolls, equipment_qualities })
    }
}

fn read_inscriptions() -> Result<HashMap<i64, InscriptionRarity>, JobsError> {
    let dataset: InscriptionDataset = yaml_serde::from_str(INSCRIPTIONS_YAML)?;
    Ok(dataset
        .inscriptions
        .into_iter()
        .filter(|(id, _)| *id > 0)
        .map(|(id, definition)| {
            let rarity = match definition.rarity {
                DatasetInscriptionRarity::Common => InscriptionRarity::Common,
                DatasetInscriptionRarity::Rare => InscriptionRarity::Rare,
                DatasetInscriptionRarity::Special => InscriptionRarity::Special,
            };
            (id, rarity)
        })
        .collect())
}

#[derive(Deserialize)]
struct InscriptionDataset {
    inscriptions: HashMap<i64, InscriptionDefinition>,
}

#[derive(Deserialize)]
struct InscriptionDefinition {
    rarity: DatasetInscriptionRarity,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum DatasetInscriptionRarity {
    Common,
    Rare,
    Special,
}

fn read_armament_rolls() -> Result<HashMap<i64, f64>, JobsError> {
    let dataset: ArmamentDataset = yaml_serde::from_str(ARMAMENTS_YAML)?;
    Ok(dataset
        .armaments
        .into_iter()
        .filter_map(|(id, definition)| {
            let maximum = definition.max_roll?;
            (id > 0 && maximum.is_finite() && maximum >= 0.0).then_some((id, maximum))
        })
        .collect())
}

#[derive(Deserialize)]
struct ArmamentDataset {
    armaments: HashMap<i64, ArmamentDefinition>,
}

#[derive(Deserialize)]
struct ArmamentDefinition {
    max_roll: Option<f64>,
}

fn read_equipment_qualities() -> Result<HashMap<i64, i64>, JobsError> {
    let dataset: EquipmentDataset = yaml_serde::from_str(EQUIPMENT_YAML)?;
    Ok(dataset
        .equipment
        .items
        .into_iter()
        .filter(|(id, _)| *id > 0)
        .map(|(id, definition)| (id, definition.rarity.quality()))
        .collect())
}

#[derive(Deserialize)]
struct EquipmentDataset {
    equipment: EquipmentCatalog,
}

#[derive(Deserialize)]
struct EquipmentCatalog {
    #[serde(rename = "item")]
    items: HashMap<i64, EquipmentDefinition>,
}

#[derive(Deserialize)]
struct EquipmentDefinition {
    rarity: EquipmentRarity,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum EquipmentRarity {
    Normal,
    Advanced,
    Elite,
    Epic,
    Legendary,
}

impl EquipmentRarity {
    const fn quality(&self) -> i64 {
        match self {
            Self::Normal => 1,
            Self::Advanced => 2,
            Self::Elite => 3,
            Self::Epic => 4,
            Self::Legendary => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_catalogs_resolve_all_three_compact_aggregation_inputs() {
        let catalogs = Catalogs::load().expect("catalogs");

        assert!(!catalogs.inscriptions.is_empty());
        assert!(catalogs.armament_max_rolls.values().all(|value| value.is_finite()));
        assert!(catalogs.equipment_qualities.values().all(|quality| *quality > 0));
    }

    #[test]
    fn equipment_rarities_map_to_game_quality_values() {
        assert_eq!(EquipmentRarity::Legendary.quality(), 5);
    }

    #[test]
    fn armament_yaml_contains_expected_maximum_rolls() {
        let maximums = read_armament_rolls().expect("armament rolls");

        assert_eq!(maximums.get(&3_001), Some(&0.035));
    }
}
