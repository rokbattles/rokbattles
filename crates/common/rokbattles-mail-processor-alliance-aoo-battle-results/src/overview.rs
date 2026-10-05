//! Extracts alliance scores and optional MVPs from six `body.kvs.max*` objects.
//!
//! Output categories cover Ark, occupation, severely wounded, provisions, healing,
//! and kill scores. Each object requires numeric `AsScore`. An absent `PlyScore`
//! or a placeholder with `Name: "-"`, zero `Score`, and no `PlyId` yields a null
//! MVP. Other entries require `PlyId`, `Name`, and `Score`; explicit null is invalid.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value, json};

use crate::content::{
    require_body_kvs, require_child_object, require_number_field, require_string_field,
    require_u64_field,
};

/// Extracts category overview records from the `body.kvs.max*` blocks.
#[derive(Debug, Default)]
pub struct OverviewExtractor;

impl OverviewExtractor {
    /// Creates an overview extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for OverviewExtractor {
    fn section(&self) -> &'static str {
        "overview"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let kvs = require_body_kvs(input)?;

        let mut section = Section::new();
        // Ark of Osiris score
        section.insert("flag_score", extract_category(kvs, "maxFlagScore")?);
        // Occupation score
        section.insert("building_score", extract_category(kvs, "maxBuildingScore")?);
        // Severely wounded units
        section.insert("be_killed_score", extract_category(kvs, "maxBeKilled")?);
        // Provisions score
        section.insert("gather_score", extract_category(kvs, "maxGatherScore")?);
        // Units healed
        section.insert("healing_score", extract_category(kvs, "maxHealingScore")?);
        // Total kills
        section.insert("killed_score", extract_category(kvs, "maxKilled")?);

        Ok(section)
    }
}

fn extract_category(kvs: &Map<String, Value>, field: &'static str) -> Result<Value, ExtractError> {
    let category = require_child_object(kvs, field)?;
    let alliance_score = require_number_field(category, "AsScore")?;
    let mvp = match category.get("PlyScore") {
        None => Value::Null,
        Some(ply_score)
            if ply_score.get("PlyId").is_none()
                && ply_score.get("Name").and_then(Value::as_str) == Some("-")
                && ply_score.get("Score").and_then(Value::as_f64) == Some(0.0) =>
        {
            Value::Null
        }
        Some(ply_score) => {
            let ply_score = ply_score
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "PlyScore", expected: "object" })?;
            let player_id = require_u64_field(ply_score, "PlyId")?;
            let player_name = require_string_field(ply_score, "Name")?;
            let score = require_number_field(ply_score, "Score")?;

            json!({
                "player_id": player_id,
                "player_name": player_name,
                "score": score,
            })
        }
    };

    Ok(json!({
        "alliance_score": alliance_score,
        "mvp": mvp
    }))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rokbattles_mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn overview_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "maxFlagScore": {
                        "AsScore": 4000,
                        "PlyScore": {
                            "PlyId": 1,
                            "Name": "A",
                            "Score": 111
                        }
                    },
                    "maxBuildingScore": {
                        "AsScore": 5000,
                        "PlyScore": {
                            "PlyId": 2,
                            "Name": "B",
                            "Score": 222
                        }
                    },
                    "maxBeKilled": {
                        "AsScore": 6000,
                        "PlyScore": {
                            "PlyId": 3,
                            "Name": "C",
                            "Score": 333
                        }
                    },
                    "maxGatherScore": {
                        "AsScore": 7000,
                        "PlyScore": {
                            "PlyId": 4,
                            "Name": "D",
                            "Score": 92.34
                        }
                    },
                    "maxHealingScore": {
                        "AsScore": 8000,
                        "PlyScore": {
                            "PlyId": 5,
                            "Name": "E",
                            "Score": 0
                        }
                    },
                    "maxKilled": {
                        "AsScore": 9000,
                        "PlyScore": {
                            "PlyId": 6,
                            "Name": "F",
                            "Score": 444
                        }
                    }
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["flag_score"]["alliance_score"], json!(4000));
        assert_eq!(fields["flag_score"]["mvp"]["player_id"], json!(1));
        assert_eq!(fields["building_score"]["mvp"]["player_name"], json!("B"));
        assert_eq!(fields["be_killed_score"]["mvp"]["score"], json!(333));
        assert_eq!(fields["gather_score"]["mvp"]["score"], json!(92.34));
        assert_eq!(fields["healing_score"]["alliance_score"], json!(8000));
        assert_eq!(fields["killed_score"]["mvp"]["player_id"], json!(6));
    }

    #[test]
    fn overview_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "maxFlagScore": {
                        "AsScore": 4000,
                        "PlyScore": {
                            "PlyId": 1,
                            "Name": "A",
                            "Score": 111
                        }
                    }
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "maxBuildingScore" }));
    }

    #[test]
    fn overview_extractor_allows_missing_ply_score() {
        let input = json!({
            "body": {
                "kvs": {
                    "maxFlagScore": { "AsScore": 0 },
                    "maxBuildingScore": { "AsScore": 0 },
                    "maxBeKilled": { "AsScore": 0 },
                    "maxGatherScore": { "AsScore": 0 },
                    "maxHealingScore": { "AsScore": 0 },
                    "maxKilled": { "AsScore": 0 }
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["flag_score"]["alliance_score"], json!(0));
        assert!(fields["flag_score"]["mvp"].is_null());
        assert!(fields["building_score"]["mvp"].is_null());
        assert!(fields["be_killed_score"]["mvp"].is_null());
        assert!(fields["gather_score"]["mvp"].is_null());
        assert!(fields["healing_score"]["mvp"].is_null());
        assert!(fields["killed_score"]["mvp"].is_null());
    }

    #[test]
    fn category_treats_placeholder_player_as_no_mvp() {
        for score in [json!(0), json!(0.0)] {
            let input = json!({
                "maxFlagScore": { "AsScore": 0, "PlyScore": { "Name": "-", "Score": score } }
            });

            let category = extract_category(input.as_object().unwrap(), "maxFlagScore").unwrap();

            assert_eq!(category, json!({ "alliance_score": 0, "mvp": null }));
        }
    }

    #[test]
    fn category_preserves_identified_player_with_placeholder_name_and_zero_score() {
        let input = json!({
            "maxFlagScore": {
                "AsScore": 0,
                "PlyScore": { "PlyId": 42, "Name": "-", "Score": 0 }
            }
        });

        let category = extract_category(input.as_object().unwrap(), "maxFlagScore").unwrap();

        assert_eq!(category["mvp"], json!({ "player_id": 42, "player_name": "-", "score": 0 }));
    }

    #[test]
    fn category_requires_player_id_for_non_placeholder_objects() {
        for ply_score in [
            json!({}),
            json!({ "Name": "Player", "Score": 0 }),
            json!({ "Name": "-", "Score": 1 }),
            json!({ "Name": "-", "Score": -1 }),
            json!({ "Name": "-", "Score": "0" }),
            json!({ "Name": "-" }),
        ] {
            let input = json!({ "maxFlagScore": { "AsScore": 0, "PlyScore": ply_score } });

            let error = extract_category(input.as_object().unwrap(), "maxFlagScore").unwrap_err();

            assert!(matches!(error, ExtractError::MissingField { field: "PlyId" }));
        }
    }

    #[test]
    fn category_rejects_placeholder_with_null_player_id() {
        let input = json!({
            "maxFlagScore": {
                "AsScore": 0,
                "PlyScore": { "PlyId": null, "Name": "-", "Score": 0 }
            }
        });

        let error = extract_category(input.as_object().unwrap(), "maxFlagScore").unwrap_err();

        assert!(matches!(error, ExtractError::InvalidFieldType { field: "PlyId", .. }));
    }

    #[test]
    fn roundtrip_overview_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["flag_score"]["alliance_score"], json!(4000));
        assert_eq!(fields["flag_score"]["mvp"]["player_id"], json!(47043938));
        assert_eq!(fields["building_score"]["mvp"]["player_name"], json!("Hellcheppapewж"));
        assert_eq!(fields["be_killed_score"]["alliance_score"], json!(79403565));
        assert_eq!(fields["gather_score"]["mvp"]["score"], json!(92.34));
        assert_eq!(fields["healing_score"]["mvp"]["score"], json!(0));
        assert_eq!(fields["killed_score"]["alliance_score"], json!(80458146));
    }

    #[test]
    fn roundtrip_overview_extracts_sample_without_mvp() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.51874049176441766435.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["flag_score"]["alliance_score"], json!(0));
        assert!(fields["flag_score"]["mvp"].is_null());
        assert!(fields["building_score"]["mvp"].is_null());
        assert!(fields["be_killed_score"]["mvp"].is_null());
        assert!(fields["gather_score"]["mvp"].is_null());
        assert!(fields["healing_score"]["mvp"].is_null());
        assert!(fields["killed_score"]["mvp"].is_null());
    }
}
