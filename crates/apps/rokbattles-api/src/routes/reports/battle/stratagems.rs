use std::{collections::HashMap, sync::OnceLock};

use mongodb::bson::{Bson, Document};
use rokbattles_bson::{
    bson_to_f64, bson_to_i64_exact, nested_array, nested_document, nested_i64_exact,
};
use serde::Deserialize;

use super::types::{
    BattleReportBattleEffects, BattleReportBattleResults, BattleReportStatisticUnit,
    BattleReportStratagem, BattleReportStratagemStatistic,
};

const DATASET: &str = include_str!("../../../../../../../datasets/stratagems.yaml");
const POLICY_V2_SOURCE: &str = "policy_v2";
const SUPPORTED_SCHEMAS: [i64; 4] = [203, 319, 819, 919];

static STRATAGEMS: OnceLock<Option<StratagemDataset>> = OnceLock::new();

#[derive(Deserialize)]
struct StratagemDataset {
    stratagems: HashMap<i64, StratagemDefinition>,
}

#[derive(Deserialize)]
struct StratagemDefinition {
    name: HashMap<String, String>,
    description: HashMap<String, String>,
    #[serde(default, rename = "override")]
    overrides: HashMap<i64, LocalizedStratagem>,
}

#[derive(Deserialize)]
struct LocalizedStratagem {
    name: HashMap<String, String>,
    description: HashMap<String, String>,
}

struct ResolvedStratagem<'a> {
    name: &'a str,
    description: &'a str,
}

pub(super) fn resolve_battle_effects(
    opponent: &Document,
    report_schema: Option<i64>,
    battle_results: &BattleReportBattleResults,
) -> Option<BattleReportBattleEffects> {
    let schema = report_schema?;
    let dataset = stratagem_dataset()?;
    if !SUPPORTED_SCHEMAS.contains(&schema) {
        return None;
    }

    let effects = nested_document(opponent, &["battle_effects"])?;
    let sender = resolve_side(
        nested_document(effects, &["sender"]),
        schema,
        battle_results.opponent.severely_wounded,
        dataset,
    );
    let opponent = resolve_side(
        nested_document(effects, &["opponent"]),
        schema,
        battle_results.sender.severely_wounded,
        dataset,
    );

    if sender.is_empty() && opponent.is_empty() {
        return None;
    }

    Some(BattleReportBattleEffects { sender, opponent })
}

fn stratagem_dataset() -> Option<&'static StratagemDataset> {
    STRATAGEMS.get_or_init(|| yaml_serde::from_str(DATASET).ok()).as_ref()
}

impl StratagemDataset {
    fn resolve(&self, schema: i64, id: i64) -> Option<ResolvedStratagem<'_>> {
        let definition = self.stratagems.get(&id)?;
        let schema_override = definition.overrides.get(&schema);

        Some(ResolvedStratagem {
            name: schema_override.map_or(&definition.name, |entry| &entry.name).get("en")?,
            description: schema_override
                .map_or(&definition.description, |entry| &entry.description)
                .get("en")?,
        })
    }
}

fn resolve_side(
    side: Option<&Document>,
    schema: i64,
    target_severely_wounded: Option<i64>,
    dataset: &StratagemDataset,
) -> Vec<BattleReportStratagem> {
    let Some(side) = side else {
        return Vec::new();
    };
    let ids = modifier_ids(side);

    ids.iter()
        .filter_map(|id| {
            let resolved = dataset.resolve(schema, *id)?;
            let statistics = statistics_for(side, *id);
            let effective_percentage =
                effective_percentage(*id, &statistics, &ids, target_severely_wounded);

            Some(BattleReportStratagem {
                id: *id,
                name: resolved.name.to_owned(),
                description: resolved.description.to_owned(),
                effective_percentage,
                statistics,
            })
        })
        .collect()
}

fn modifier_ids(side: &Document) -> Vec<i64> {
    nested_array(side, &["modifier_sources"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .filter(|source| source.get_str("source").ok() == Some(POLICY_V2_SOURCE))
        .filter_map(|source| nested_array(source, &["ids"]))
        .flatten()
        .filter_map(bson_to_i64_exact)
        .collect()
}

fn statistics_for(side: &Document, id: i64) -> Vec<BattleReportStratagemStatistic> {
    let Some(source) = nested_array(side, &["statistics"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .find(|source| {
            source.get_str("source").ok() == Some(POLICY_V2_SOURCE)
                && nested_i64_exact(source, &["id"]) == Some(id)
        })
    else {
        return Vec::new();
    };

    nested_array(source, &["stats"])
        .into_iter()
        .flatten()
        .filter_map(Bson::as_document)
        .filter_map(map_statistic)
        .collect()
}

fn map_statistic(statistic: &Document) -> Option<BattleReportStratagemStatistic> {
    let key = statistic.get_str("key").ok()?.to_owned();
    let value = statistic.get("value")?.clone();
    let (scale, unit) = statistic_scale_and_unit(&key);
    let display_value = bson_to_f64(&value).map(|value| value / scale);

    Some(BattleReportStratagemStatistic { key, value, display_value, unit })
}

fn statistic_scale_and_unit(key: &str) -> (f64, Option<BattleReportStatisticUnit>) {
    use BattleReportStatisticUnit::{Number, Percent};

    match key {
        "Atk" | "DamageRaise" => (100.0, Some(Percent)),
        "BadHurt" | "ExtraBadHurt" | "KillTimes" | "Kill" | "BeDmgReduceTimes" | "HealTimes"
        | "Heal" | "Dead" | "SkillDmgRaiseTimes" | "SeverelyWounded" | "KvkLostT5" => {
            (1.0, Some(Number))
        }
        _ => (1.0, None),
    }
}

fn effective_percentage(
    id: i64,
    statistics: &[BattleReportStratagemStatistic],
    equipped_ids: &[i64],
    target_severely_wounded: Option<i64>,
) -> Option<f64> {
    match id {
        1 => statistic_display_value(statistics, "Atk"),
        2 => decimation_percentage(statistics, equipped_ids, target_severely_wounded),
        26 => statistic_display_value(statistics, "DamageRaise"),
        _ => None,
    }
}

fn statistic_display_value(
    statistics: &[BattleReportStratagemStatistic],
    key: &str,
) -> Option<f64> {
    statistics.iter().find(|statistic| statistic.key == key)?.display_value
}

fn decimation_percentage(
    statistics: &[BattleReportStratagemStatistic],
    equipped_ids: &[i64],
    target_severely_wounded: Option<i64>,
) -> Option<f64> {
    let extra = statistics
        .iter()
        .find(|statistic| statistic.key == "ExtraBadHurt")
        .and_then(|statistic| bson_to_i64_exact(&statistic.value))?;
    let base = target_severely_wounded?.checked_sub(extra)?;
    if base <= 0 || extra <= 0 {
        return None;
    }

    let candidates: &[i64] = if equipped_ids.contains(&14) { &[6, 3] } else { &[4, 2] };
    candidates
        .iter()
        .copied()
        .find(|percentage| ceil_percentage(base, *percentage) == Some(extra))
        .map(|percentage| percentage as f64)
}

fn ceil_percentage(value: i64, percentage: i64) -> Option<i64> {
    let product = i128::from(value).checked_mul(i128::from(percentage))?;
    i64::try_from((product + 99) / 100).ok()
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    fn battle_results(sender_severely_wounded: i64) -> BattleReportBattleResults {
        BattleReportBattleResults {
            sender: super::super::types::BattleReportBattleResult {
                severely_wounded: Some(sender_severely_wounded),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn stratagem_dataset_deserializes_and_resolves_schema_override() {
        let dataset: StratagemDataset = yaml_serde::from_str(DATASET).expect("valid dataset");
        let common = dataset.resolve(819, 3).expect("common stratagem");
        let kvk_three = dataset.resolve(203, 3).expect("KVK 3 override");

        assert!(common.description.contains("20% more counterattack damage"));
        assert!(kvk_three.description.contains("16% more counterattack damage"));
    }

    #[test]
    fn stratagem_dataset_contains_every_supported_locale() {
        const LOCALES: [&str; 18] = [
            "ar", "de", "en", "es", "fr", "id", "it", "ja", "ko", "ms", "pl", "pt", "ru", "th",
            "tr", "vi", "zh_CN", "zh_TW",
        ];

        fn contains_every_locale(localized: &HashMap<String, String>) -> bool {
            localized.len() == LOCALES.len()
                && LOCALES.iter().all(|locale| localized.contains_key(*locale))
        }

        let dataset: StratagemDataset = yaml_serde::from_str(DATASET).expect("valid dataset");
        let all_localized = dataset.stratagems.values().all(|stratagem| {
            contains_every_locale(&stratagem.name)
                && contains_every_locale(&stratagem.description)
                && stratagem.overrides.values().all(|schema_override| {
                    contains_every_locale(&schema_override.name)
                        && contains_every_locale(&schema_override.description)
                })
        });

        assert!(all_localized);
    }

    #[test]
    fn stratagem_dataset_localizations_only_use_newline_control_characters() {
        fn is_clean(localized: &HashMap<String, String>) -> bool {
            localized.values().all(|value| {
                !value.chars().any(|character| character != '\n' && character.is_control())
            })
        }

        let dataset: StratagemDataset = yaml_serde::from_str(DATASET).expect("valid dataset");
        let all_clean = dataset.stratagems.values().all(|stratagem| {
            is_clean(&stratagem.name)
                && is_clean(&stratagem.description)
                && stratagem.overrides.values().all(|schema_override| {
                    is_clean(&schema_override.name) && is_clean(&schema_override.description)
                })
        });

        assert!(all_clean);
    }

    #[test]
    fn resolves_both_koab_sides_and_calculates_decimation_percentage() {
        let opponent = doc! {
            "battle_effects": {
                "sender": {
                    "modifier_sources": [{ "source": "policy_v2", "ids": [1_i64, 13_i64] }],
                    "statistics": [{
                        "source": "policy_v2",
                        "id": 1_i64,
                        "stats": [
                            { "key": "BadHurt", "value": 22_588_870_i64 },
                            { "key": "Atk", "value": 2_100_i64 },
                        ],
                    }],
                },
                "opponent": {
                    "modifier_sources": [{ "source": "policy_v2", "ids": [2_i64, 14_i64] }],
                    "statistics": [{
                        "source": "policy_v2",
                        "id": 2_i64,
                        "stats": [{ "key": "ExtraBadHurt", "value": 719_i64 }],
                    }],
                },
            },
        };

        let effects = resolve_battle_effects(&opponent, Some(819), &battle_results(12_691))
            .expect("KOAB effects");

        assert_eq!(effects.sender[0].name, "Blood-Tempered");
        assert_eq!(effects.sender[0].effective_percentage, Some(21.0));
        assert_eq!(effects.sender[0].statistics[1].display_value, Some(21.0));
        assert_eq!(effects.opponent[0].name, "Decimation");
        assert_eq!(effects.opponent[0].effective_percentage, Some(6.0));
        assert_eq!(effects.opponent[0].statistics[0].display_value, Some(719.0));
    }

    #[test]
    fn keeps_one_sided_koab_effects_aligned_for_the_api() {
        let opponent = doc! {
            "battle_effects": {
                "sender": {
                    "modifier_sources": [],
                    "statistics": [],
                },
                "opponent": {
                    "modifier_sources": [{ "source": "policy_v2", "ids": [20_i64] }],
                    "statistics": [],
                },
            },
        };

        let effects = resolve_battle_effects(&opponent, Some(819), &Default::default())
            .expect("one-sided effects");

        assert!(effects.sender.is_empty());
        assert_eq!(effects.opponent[0].name, "Breakneck");
    }

    #[test]
    fn omits_empty_and_unsupported_battle_effects() {
        let opponent = doc! {
            "battle_effects": {
                "sender": { "modifier_sources": [], "statistics": [] },
                "opponent": {
                    "modifier_sources": [{ "source": "policy_v2", "ids": [20_i64] }],
                    "statistics": [],
                },
            },
        };

        assert!(resolve_battle_effects(&opponent, Some(810), &Default::default()).is_none());

        let empty = doc! {
            "battle_effects": {
                "sender": { "modifier_sources": [], "statistics": [] },
                "opponent": { "modifier_sources": [], "statistics": [] },
            },
        };
        assert!(resolve_battle_effects(&empty, Some(819), &Default::default()).is_none());
    }
}
