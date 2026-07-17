use std::{borrow::Borrow, collections::HashMap, fmt, sync::OnceLock};

use core_bson::{nested_bool, nested_i64_exact};
use mongodb::bson::Document;
use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};

const DATASET: &str = include_str!("../../../../../../../datasets/structures.yaml");

static ICONS: OnceLock<Option<StructureIconDataset>> = OnceLock::new();

#[derive(Deserialize)]
struct StructureIconDataset {
    structures: Structures,
}

#[derive(Deserialize)]
struct Structures {
    aliases: HashMap<i64, i64>,
    alliance: AllianceStructures,
    building: BuildingStructures,
}

#[derive(Deserialize)]
struct AllianceStructures {
    default: String,
    #[serde(flatten)]
    items: HashMap<NumericKey, AllianceStructure>,
}

#[derive(Deserialize)]
struct AllianceStructure {
    sprite: Vec<String>,
    #[serde(default, rename = "override")]
    overrides: AllianceOverrides,
}

#[derive(Default, Deserialize)]
struct AllianceOverrides {
    turret: Option<SpriteList>,
    outpost: Option<SpriteList>,
}

#[derive(Deserialize)]
struct BuildingStructures {
    default: String,
    #[serde(flatten)]
    items: HashMap<NumericKey, BuildingStructure>,
}

#[derive(Deserialize)]
struct BuildingStructure {
    sprite: Vec<String>,
    #[serde(default, rename = "override")]
    overrides: HashMap<i64, SpriteList>,
}

#[derive(Deserialize)]
struct SpriteList {
    sprite: Vec<String>,
}

#[derive(Eq, Hash, PartialEq)]
struct NumericKey(i64);

impl Borrow<i64> for NumericKey {
    fn borrow(&self) -> &i64 {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NumericKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NumericKeyVisitor;

        impl Visitor<'_> for NumericKeyVisitor {
            type Value = NumericKey;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an integer mapping key")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(NumericKey(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                i64::try_from(value).map(NumericKey).map_err(de::Error::custom)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map(NumericKey).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(NumericKeyVisitor)
    }
}

pub(super) fn resolve_avatar_override(
    participant: &Document,
    report_schema: Option<i64>,
) -> Option<String> {
    let icons = icon_dataset()?;

    let sprite = if let Some(structure_id) = nested_i64_exact(participant, &["structure_id"]) {
        icons.structure_sprite(structure_id, report_schema)
    } else {
        if nested_i64_exact(participant, &["character_type"]) != Some(7) {
            return None;
        }

        icons.alliance_sprite(
            nested_i64_exact(participant, &["alliance_building_id"]),
            nested_bool(participant, &["is_turret"]) == Some(true),
            nested_bool(participant, &["is_outpost"]) == Some(true),
        )
    }?;

    Some(format!("https://cdn.rokbattles.com/game/sprites/{sprite}"))
}

fn icon_dataset() -> Option<&'static StructureIconDataset> {
    ICONS.get_or_init(|| yaml_serde::from_str::<StructureIconDataset>(DATASET).ok()).as_ref()
}

impl StructureIconDataset {
    fn structure_sprite(&self, structure_id: i64, report_schema: Option<i64>) -> Option<&str> {
        let converted_schema = report_schema
            .map(|schema| self.structures.aliases.get(&schema).copied().unwrap_or(schema));

        let Some(icon) = self.structures.building.items.get(&structure_id) else {
            return Some(&self.structures.building.default);
        };

        if let Some(icon_override) = converted_schema.and_then(|schema| icon.overrides.get(&schema))
        {
            return first_sprite(&icon_override.sprite);
        }

        first_sprite(&icon.sprite)
    }

    fn alliance_sprite(
        &self,
        building_id: Option<i64>,
        is_turret: bool,
        is_outpost: bool,
    ) -> Option<&str> {
        let alliance = &self.structures.alliance;
        let Some(icon) = building_id.and_then(|id| alliance.items.get(&id)) else {
            return Some(&alliance.default);
        };

        if is_turret && let Some(icon_override) = &icon.overrides.turret {
            return first_sprite(&icon_override.sprite);
        }
        if is_outpost && let Some(icon_override) = &icon.overrides.outpost {
            return first_sprite(&icon_override.sprite);
        }
        first_sprite(&icon.sprite)
    }
}

fn first_sprite(sprites: &[String]) -> Option<&str> {
    sprites.first().map(String::as_str).filter(|sprite| !sprite.is_empty())
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::{DATASET, StructureIconDataset, resolve_avatar_override};

    fn resolved_url(participant: mongodb::bson::Document, schema: Option<i64>) -> Option<String> {
        resolve_avatar_override(&participant, schema)
    }

    #[test]
    fn structures_dataset_deserializes() {
        yaml_serde::from_str::<StructureIconDataset>(DATASET).expect("valid structures dataset");
    }

    #[test]
    fn structure_109_uses_schema_specific_sprite() {
        let url = resolved_url(doc! { "structure_id": 109_i64 }, Some(10_002));

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconGVGBuilding16.png")
        );
    }

    #[test]
    fn structure_38_converts_report_schema_before_lookup() {
        let url = resolved_url(doc! { "structure_id": 38_i64 }, Some(814));

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconT7Altar.png")
        );
    }

    #[test]
    fn structure_52_uses_matching_schema_sprite() {
        let url = resolved_url(doc! { "structure_id": 52_i64 }, Some(412));

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconPass.png")
        );
    }

    #[test]
    fn unknown_structure_uses_fallback_sprite() {
        let url = resolved_url(doc! { "structure_id": 999_999_i64 }, Some(999_999));

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconT1Altar.png")
        );
    }

    #[test]
    fn alliance_fortress_uses_default_building_sprite() {
        let url =
            resolved_url(doc! { "character_type": 7_i64, "alliance_building_id": 3_i64 }, None);

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconAlliMainHall.png")
        );
    }

    #[test]
    fn alliance_building_types_cover_every_sprite_outcome() {
        let actual = [1_i64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 999].map(|id| {
            resolved_url(doc! { "character_type": 7_i64, "alliance_building_id": id }, None)
        });
        let expected = [
            "img_iconAlliFlag.png",
            "img_iconAlliMainHall.png",
            "img_iconAlliMainHall.png",
            "img_iconSuperFood.png",
            "img_iconSuperWood.png",
            "img_iconSuperStone.png",
            "img_iconSuperGold.png",
            "img_iconAlliMainHall.png",
            "img_iconAlliMainHall.png",
            "img_iconAlliFlag.png",
            "img_iconAlliMainMuMa2.png",
            "img_iconAlliMainHall.png",
        ]
        .map(|sprite| Some(format!("https://cdn.rokbattles.com/game/sprites/{sprite}")));

        assert_eq!(actual, expected);
    }

    #[test]
    fn alliance_flag_turret_takes_precedence_over_outpost() {
        let url = resolved_url(
            doc! {
                "character_type": 7_i64,
                "alliance_building_id": 1_i64,
                "is_turret": true,
                "is_outpost": true,
            },
            None,
        );

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconAlliArrow.png")
        );
    }

    #[test]
    fn alliance_flag_outpost_uses_outpost_sprite() {
        let url = resolved_url(
            doc! {
                "character_type": 7_i64,
                "alliance_building_id": 1_i64,
                "is_outpost": true,
            },
            None,
        );

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconKVKS7Building5.png")
        );
    }

    #[test]
    fn alliance_character_type_without_id_uses_default_sprite() {
        let url = resolved_url(doc! { "character_type": 7_i64 }, None);

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconAlliMainHall.png")
        );
    }

    #[test]
    fn alliance_id_without_matching_character_type_does_not_override_avatar() {
        let resolved = resolve_avatar_override(
            &doc! { "character_type": 1_i64, "alliance_building_id": 1_i64 },
            None,
        );

        assert!(resolved.is_none());
    }

    #[test]
    fn structure_takes_precedence_over_alliance_building() {
        let url = resolved_url(
            doc! {
                "structure_id": 52_i64,
                "character_type": 7_i64,
                "alliance_building_id": 1_i64,
            },
            Some(412),
        );

        assert_eq!(
            url.as_deref(),
            Some("https://cdn.rokbattles.com/game/sprites/img_iconPass.png")
        );
    }
}
