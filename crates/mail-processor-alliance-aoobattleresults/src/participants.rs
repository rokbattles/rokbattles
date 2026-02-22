//! Participants extractor for AllianceAOOBattleResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, indexed_array_values};
use serde_json::{Value, json};

use crate::content::{require_body_kvs, require_number_field, require_string_field};

/// Extracts individual score lines from `body.kvs.plyRanks`.
#[derive(Debug, Default)]
pub struct ParticipantsExtractor;

impl ParticipantsExtractor {
    /// Create a new participants extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for ParticipantsExtractor {
    fn section(&self) -> &'static str {
        "participants"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let kvs = require_body_kvs(input)?;
        let ply_ranks = kvs
            .get("plyRanks")
            .ok_or(ExtractError::MissingField { field: "plyRanks" })?;
        let ply_ranks = indexed_array_values(ply_ranks, "plyRanks")?;

        let mut participants = Vec::with_capacity(ply_ranks.len());
        for entry in ply_ranks {
            let entry = entry.as_object().ok_or(ExtractError::InvalidFieldType {
                field: "plyRanks",
                expected: "object",
            })?;

            let player_name = require_string_field(entry, "Name")?;
            let individual_points = require_number_field(entry, "Score")?;
            let building_score = require_number_field(entry, "BuildingScore")?;
            let gather_score = require_number_field(entry, "GatherScore")?;
            let kill_score = require_number_field(entry, "KillScore")?;
            let flag_score = require_number_field(entry, "FlagScore")?;

            participants.push(json!({
                "player_name": player_name,
                "individual_points": individual_points,
                "building_score": building_score,
                "gather_score": gather_score,
                "kill_score": kill_score,
                "flag_score": flag_score,
            }));
        }

        Ok(Section::from_array(participants))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn participants_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "plyRanks": [
                        1,
                        {
                            "Name": "Tester",
                            "Score": -1,
                            "BuildingScore": 10,
                            "GatherScore": 2.5,
                            "KillScore": 99,
                            "FlagScore": 3
                        }
                    ]
                }
            }
        });

        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0]["player_name"], json!("Tester"));
        assert_eq!(participants[0]["individual_points"], json!(-1));
        assert_eq!(participants[0]["building_score"], json!(10));
        assert_eq!(participants[0]["gather_score"], json!(2.5));
        assert_eq!(participants[0]["kill_score"], json!(99));
        assert_eq!(participants[0]["flag_score"], json!(3));
    }

    #[test]
    fn participants_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "plyRanks": [
                        1,
                        {
                            "Name": "Tester",
                            "BuildingScore": 10,
                            "GatherScore": 2,
                            "KillScore": 99,
                            "FlagScore": 3
                        }
                    ]
                }
            }
        });

        let extractor = ParticipantsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "Score" }));
    }

    #[test]
    fn roundtrip_participants_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let participants = section.array().expect("participants");

        assert_eq!(participants[0]["player_name"], json!("jën x ƒaräɀ"));
        assert_eq!(participants[0]["individual_points"], json!(188637));
        assert_eq!(participants[0]["building_score"], json!(2745));
        assert_eq!(participants[0]["gather_score"], json!(0));
        assert_eq!(participants[0]["kill_score"], json!(4174626));
        assert_eq!(participants[0]["flag_score"], json!(0));
        assert_eq!(participants[30]["individual_points"], json!(-1));
    }
}
