//! Battle modifier and effect-statistic parsing.

use mail_sdk::{ExtractError, indexed_array_values, require_string_field, require_u64_field};
use serde_json::{Map, Value, json};

/// Normalizes the sender and opponent effects attached to one attack.
pub(crate) fn extract_battle_effects(attack: &Map<String, Value>) -> Result<Value, ExtractError> {
    let sender = extract_side(attack, "SelfKvkModBuffIds", "SelfEffectStats")?;
    let opponent = extract_side(attack, "OpsKvkModBuffIds", "OpsEffectStats")?;

    Ok(json!({ "sender": sender, "opponent": opponent }))
}

fn extract_side(
    attack: &Map<String, Value>,
    modifier_field: &'static str,
    statistics_field: &'static str,
) -> Result<Value, ExtractError> {
    let modifier_sources = extract_modifier_sources(attack.get(modifier_field), modifier_field)?;
    let statistics = extract_statistics(attack.get(statistics_field), statistics_field)?;

    Ok(json!({
        "modifier_sources": modifier_sources,
        "statistics": statistics,
    }))
}

fn extract_modifier_sources(
    value: Option<&Value>,
    field: &'static str,
) -> Result<Vec<Value>, ExtractError> {
    optional_indexed_values(value, field)?
        .into_iter()
        .map(|entry| {
            let entry = entry
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field, expected: "object" })?;
            let source = require_string_field(entry, "Source")?;
            let ids = extract_modifier_ids(entry.get("Ids"))?;

            Ok(json!({ "source": source, "ids": ids }))
        })
        .collect()
}

fn extract_modifier_ids(value: Option<&Value>) -> Result<Vec<u64>, ExtractError> {
    optional_indexed_values(value, "Ids")?
        .into_iter()
        .map(|id| {
            id.as_u64().ok_or(ExtractError::InvalidFieldType {
                field: "Ids",
                expected: "unsigned integer",
            })
        })
        .collect()
}

fn extract_statistics(
    value: Option<&Value>,
    field: &'static str,
) -> Result<Vec<Value>, ExtractError> {
    optional_indexed_values(value, field)?
        .into_iter()
        .map(|entry| {
            let entry = entry
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field, expected: "object" })?;
            let source = require_string_field(entry, "Source")?;
            let id = require_u64_field(entry, "Id")?;
            let stats = extract_stat_values(entry.get("Stats"))?;

            Ok(json!({ "source": source, "id": id, "stats": stats }))
        })
        .collect()
}

fn extract_stat_values(value: Option<&Value>) -> Result<Vec<Value>, ExtractError> {
    optional_indexed_values(value, "Stats")?
        .into_iter()
        .map(|stat| {
            let stat = stat
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "Stats", expected: "object" })?;
            let key = require_string_field(stat, "K")?;
            let value = stat.get("V").ok_or(ExtractError::MissingField { field: "V" })?;

            Ok(json!({ "key": key, "value": value }))
        })
        .collect()
}

fn optional_indexed_values<'a>(
    value: Option<&'a Value>,
    field: &'static str,
) -> Result<Vec<&'a Value>, ExtractError> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(value) => indexed_array_values(value, field),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    fn load_sample(mail_id: &str) -> Value {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../../../samples/Battle/Persistent.Mail.{mail_id}.json"));
        let input = fs::read_to_string(sample_path).expect("read sample");
        serde_json::from_str(&input).expect("parse sample")
    }

    fn attack<'a>(sample: &'a Value, attack_id: &str) -> &'a Map<String, Value> {
        sample["body"]["content"]["Attacks"][attack_id].as_object().expect("attack object")
    }

    #[test]
    fn extract_battle_effects_decodes_both_sides_and_preserves_values() {
        let input = json!({
            "SelfKvkModBuffIds": [
                1,
                { "Source": "policy_v2", "Ids": [1, 24, 2, 22] }
            ],
            "OpsKvkModBuffIds": [
                { "Source": "custom", "Ids": [9, 10, 11] }
            ],
            "SelfEffectStats": [
                1,
                {
                    "Source": "unit_skill",
                    "Id": 77,
                    "Stats": [1, { "K": "FutureValue", "V": { "amount": 1.5 } }]
                }
            ],
            "OpsEffectStats": [
                1,
                {
                    "Source": "policy_v2",
                    "Id": 2,
                    "Stats": [1, { "K": "ExtraBadHurt", "V": 719 }]
                }
            ]
        });

        let effects = extract_battle_effects(input.as_object().expect("attack object"))
            .expect("extract effects");

        assert_eq!(
            effects,
            json!({
                "sender": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [24, 22] }
                    ],
                    "statistics": [
                        {
                            "source": "unit_skill",
                            "id": 77,
                            "stats": [
                                { "key": "FutureValue", "value": { "amount": 1.5 } }
                            ]
                        }
                    ]
                },
                "opponent": {
                    "modifier_sources": [
                        { "source": "custom", "ids": [9, 10, 11] }
                    ],
                    "statistics": [
                        {
                            "source": "policy_v2",
                            "id": 2,
                            "stats": [
                                { "key": "ExtraBadHurt", "value": 719 }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn extract_battle_effects_defaults_missing_fields_to_empty_arrays() {
        let effects = extract_battle_effects(&Map::new()).expect("extract effects");

        assert_eq!(
            effects,
            json!({
                "sender": { "modifier_sources": [], "statistics": [] },
                "opponent": { "modifier_sources": [], "statistics": [] }
            })
        );
    }

    #[test]
    fn extract_battle_effects_defaults_null_and_empty_fields_to_empty_arrays() {
        let input = json!({
            "SelfKvkModBuffIds": null,
            "OpsKvkModBuffIds": [],
            "SelfEffectStats": [],
            "OpsEffectStats": null
        });

        let effects = extract_battle_effects(input.as_object().expect("attack object"))
            .expect("extract effects");

        assert_eq!(
            effects,
            json!({
                "sender": { "modifier_sources": [], "statistics": [] },
                "opponent": { "modifier_sources": [], "statistics": [] }
            })
        );
    }

    #[test]
    fn extract_battle_effects_keeps_source_without_ids() {
        let input = json!({
            "SelfKvkModBuffIds": [1, { "Source": "policy_v2" }]
        });

        let effects = extract_battle_effects(input.as_object().expect("attack object"))
            .expect("extract effects");

        assert_eq!(
            effects["sender"]["modifier_sources"],
            json!([{ "source": "policy_v2", "ids": [] }])
        );
    }

    #[test]
    fn extract_battle_effects_rejects_non_array_container() {
        let input = json!({ "OpsEffectStats": {} });

        let error = extract_battle_effects(input.as_object().expect("attack object"))
            .expect_err("reject malformed effects");

        assert_eq!(
            error,
            ExtractError::InvalidFieldType { field: "OpsEffectStats", expected: "array" }
        );
    }

    #[test]
    fn extract_battle_effects_rejects_non_array_ids() {
        let input = json!({
            "OpsKvkModBuffIds": [
                { "Source": "policy_v2", "Ids": {} }
            ]
        });

        let error = extract_battle_effects(input.as_object().expect("attack object"))
            .expect_err("reject malformed ids");

        assert_eq!(error, ExtractError::InvalidFieldType { field: "Ids", expected: "array" });
    }

    #[test]
    fn extract_battle_effects_rejects_stat_without_value() {
        let input = json!({
            "SelfEffectStats": [
                1,
                {
                    "Source": "unit_skill",
                    "Id": 77,
                    "Stats": [1, { "K": "MissingValue" }]
                }
            ]
        });

        let error = extract_battle_effects(input.as_object().expect("attack object"))
            .expect_err("reject missing value");

        assert_eq!(error, ExtractError::MissingField { field: "V" });
    }

    #[test]
    fn extract_battle_effects_reads_schema_819_target() {
        let sample = load_sample("13168813178320161112");

        let effects =
            extract_battle_effects(attack(&sample, "180150801")).expect("extract target effects");

        assert_eq!(
            effects,
            json!({
                "sender": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [24, 22, 28, 30, 31, 23] }
                    ],
                    "statistics": []
                },
                "opponent": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [11, 19, 7, 12, 2, 14] }
                    ],
                    "statistics": [
                        {
                            "source": "policy_v2",
                            "id": 2,
                            "stats": [
                                { "key": "ExtraBadHurt", "value": 719 }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn extract_battle_effects_reads_schema_819_comparison() {
        let sample = load_sample("47051167177023603224");

        let effects = extract_battle_effects(attack(&sample, "560025001"))
            .expect("extract comparison effects");

        assert_eq!(
            effects,
            json!({
                "sender": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [] }
                    ],
                    "statistics": []
                },
                "opponent": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [6, 19, 2] }
                    ],
                    "statistics": [
                        {
                            "source": "policy_v2",
                            "id": 2,
                            "stats": [
                                { "key": "ExtraBadHurt", "value": 662 }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn extract_battle_effects_reads_empty_schema_810_baseline() {
        let sample = load_sample("32282022178380122828");

        let effects =
            extract_battle_effects(attack(&sample, "585281501")).expect("extract baseline effects");

        assert_eq!(
            effects,
            json!({
                "sender": { "modifier_sources": [], "statistics": [] },
                "opponent": { "modifier_sources": [], "statistics": [] }
            })
        );
    }
}
