//! Participants parser for AllianceAOORegistration mail.

use mail_processor_sdk::{
    ExtractError, Extractor, Section, indexed_array_values, require_number_field,
    require_string_field, require_u64_field,
};
use serde_json::{Value, json};

use crate::content::require_body_kvs;

const PARTICIPANT_FIELDS: [&str; 3] = ["resevePly", "commanderPly", "allowPly"];

/// Pulls registration participant entries from all registration role arrays.
#[derive(Debug, Default)]
pub struct ParticipantsExtractor;

impl ParticipantsExtractor {
    /// Creates a participants extractor.
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
        let mut participants = Vec::new();

        for field in PARTICIPANT_FIELDS {
            let entries = kvs.get(field).ok_or(ExtractError::MissingField { field })?;
            for entry in indexed_array_values(entries, field)? {
                participants.push(extract_participant(entry, field)?);
            }
        }

        Ok(Section::from_array(participants))
    }
}

fn extract_participant(entry: &Value, field: &'static str) -> Result<Value, ExtractError> {
    let entry =
        entry.as_object().ok_or(ExtractError::InvalidFieldType { field, expected: "object" })?;

    Ok(json!({
        "player_name": require_string_field(entry, "Name")?,
        "player_id": require_u64_field(entry, "PlyId")?,
        "power": require_number_field(entry, "Power")?,
        "role": require_u64_field(entry, "memberType")?,
    }))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn participants_extractor_combines_registration_arrays() {
        let input = json!({
            "body": {
                "kvs": {
                    "resevePly": [
                        1,
                        {
                            "Name": "Reserve",
                            "PlyId": 10,
                            "Power": 1000,
                            "memberType": 2
                        }
                    ],
                    "commanderPly": [
                        1,
                        {
                            "Name": "Commander",
                            "PlyId": 20,
                            "Power": 2000,
                            "memberType": 3
                        }
                    ],
                    "allowPly": [
                        1,
                        {
                            "Name": "Allowed",
                            "PlyId": 30,
                            "Power": 3000,
                            "memberType": 1
                        }
                    ]
                }
            }
        });

        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 3);
        assert_eq!(participants[0]["player_name"], json!("Reserve"));
        assert_eq!(participants[0]["player_id"], json!(10));
        assert_eq!(participants[0]["power"], json!(1000));
        assert_eq!(participants[0]["role"], json!(2));
        assert_eq!(participants[1]["player_name"], json!("Commander"));
        assert_eq!(participants[2]["player_name"], json!("Allowed"));
    }

    #[test]
    fn participants_extractor_rejects_missing_participant_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "resevePly": [],
                    "commanderPly": [],
                    "allowPly": [
                        1,
                        {
                            "Name": "Allowed",
                            "PlyId": 30,
                            "Power": 3000
                        }
                    ]
                }
            }
        });

        let extractor = ParticipantsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "memberType" }));
    }

    #[test]
    fn roundtrip_participants_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.1087260861777680.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 30);
        assert_eq!(participants[0]["player_name"], json!("ˢJianLaars 命"));
        assert_eq!(participants[0]["player_id"], json!(41377732));
        assert_eq!(participants[0]["power"], json!(104670909));
        assert_eq!(participants[0]["role"], json!(3));
        assert_eq!(participants[5]["player_name"], json!("人Øgeday人"));
        assert_eq!(participants[29]["player_name"], json!("ᅠᅠ     ᅠ Swalsh"));
    }
}
