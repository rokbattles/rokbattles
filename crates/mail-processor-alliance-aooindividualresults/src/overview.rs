//! Overview extractor for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_object};
use serde_json::{Map, Value, json};

use crate::content::{optional_child_object, optional_child_object_or_empty_array};

/// Extracts the top score leaderboard entry and aggregate totals from `body.kvs`.
#[derive(Debug, Default)]
pub struct OverviewExtractor;

impl OverviewExtractor {
    /// Create a new overview extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for OverviewExtractor {
    fn section(&self) -> &'static str {
        "overview"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let root = require_object(input)?;
        let body = require_child_object(root, "body")?;
        let kvs = require_child_object(body, "kvs")?;
        let total_score_rank = require_child_object(kvs, "TotalScoreRank")?;
        let rank = require_u64_field(total_score_rank, "Rank")?;
        let info = optional_child_object(total_score_rank, "Info")?;
        let (player_name, player_id, score) = match info {
            Some(info) => (
                Value::String(require_string_field(info, "Name")?),
                Value::from(require_u64_field(info, "PlyId")?),
                Value::from(require_u64_field(info, "Score")?),
            ),
            None => (Value::Null, Value::Null, Value::Null),
        };
        let total_results = match optional_child_object(kvs, "FightReport")? {
            Some(fight_report) => match optional_child_object_or_empty_array(fight_report, "Stat")?
            {
                Some(stat) => match optional_child_object(stat, "WildBattleStat")? {
                    Some(wild_battle_stat) => {
                        let battles = require_u64_field(wild_battle_stat, "BattleCnt")?;
                        let kill_points = require_u64_field(wild_battle_stat, "KillScore")?;
                        let severely_wounded =
                            require_u64_field(wild_battle_stat, "BeKilledScore")?;
                        json!({
                            "battles": battles,
                            "kill_points": kill_points,
                            "severely_wounded": severely_wounded,
                        })
                    }
                    None => Value::Null,
                },
                None => Value::Null,
            },
            None => Value::Null,
        };

        let mut section = Section::new();
        section.insert("player_name", player_name);
        section.insert("player_id", player_id);
        section.insert("score", score);
        section.insert("rank", Value::from(rank));
        section.insert("total_results", total_results);
        Ok(section)
    }
}

fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "object",
    })
}

fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "unsigned integer",
    })
}

fn require_string_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<String, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or(ExtractError::InvalidFieldType {
            field,
            expected: "string",
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn overview_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "TotalScoreRank": {
                        "Info": {
                            "Name": "Player One",
                            "PlyId": 123,
                            "Score": 4567
                        },
                        "Rank": 8
                    },
                    "FightReport": {
                        "Stat": {
                            "WildBattleStat": {
                                "BattleCnt": 99,
                                "KillScore": 10001,
                                "BeKilledScore": 5000
                            }
                        }
                    }
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["player_name"], json!("Player One"));
        assert_eq!(fields["player_id"], json!(123));
        assert_eq!(fields["score"], json!(4567));
        assert_eq!(fields["rank"], json!(8));
        assert_eq!(fields["total_results"]["battles"], json!(99));
        assert_eq!(fields["total_results"]["kill_points"], json!(10001));
        assert_eq!(fields["total_results"]["severely_wounded"], json!(5000));
    }

    #[test]
    fn overview_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "TotalScoreRank": {
                        "Info": {
                            "Name": "Player One",
                            "PlyId": 123,
                            "Score": 4567
                        }
                    }
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "Rank" }));
    }

    #[test]
    fn overview_extractor_allows_missing_info_and_fight_report() {
        let input = json!({
            "body": {
                "kvs": {
                    "TotalScoreRank": {
                        "Rank": 0
                    }
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["rank"], json!(0));
        assert!(fields["player_name"].is_null());
        assert!(fields["player_id"].is_null());
        assert!(fields["score"].is_null());
        assert!(fields["total_results"].is_null());
    }

    #[test]
    fn roundtrip_overview_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["player_name"], json!("Grigvar"));
        assert_eq!(fields["player_id"], json!(71738515));
        assert_eq!(fields["score"], json!(123123));
        assert_eq!(fields["rank"], json!(12));
        assert_eq!(fields["total_results"]["battles"], json!(1426));
        assert_eq!(fields["total_results"]["kill_points"], json!(52552570));
        assert_eq!(fields["total_results"]["severely_wounded"], json!(53165020));
    }

    #[test]
    fn roundtrip_overview_extracts_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6890312417293500508.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["rank"], json!(0));
        assert!(fields["player_name"].is_null());
        assert!(fields["player_id"].is_null());
        assert!(fields["score"].is_null());
        assert!(fields["total_results"].is_null());
    }

    #[test]
    fn roundtrip_overview_extracts_empty_stat_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.91536773174395176822.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["player_name"], json!("KEMO farm 2"));
        assert_eq!(fields["player_id"], json!(186296796));
        assert_eq!(fields["score"], json!(2159));
        assert_eq!(fields["rank"], json!(6));
        assert!(fields["total_results"].is_null());
    }
}
