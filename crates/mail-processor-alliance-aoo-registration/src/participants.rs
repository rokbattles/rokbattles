//! Participants parser for AllianceAOORegistration mail.

use mail_processor_sdk::{
    ExtractError, Extractor, Section, indexed_array_values, optional_u64_field,
    require_number_field, require_string_field, require_u64_field,
};
use serde_json::{Value, json};

use crate::content::require_body_kvs;

#[derive(Clone, Copy, Debug)]
struct ParticipantField {
    name: &'static str,
    inferred_role: u64,
}

const PARTICIPANT_FIELDS: [ParticipantField; 3] = [
    ParticipantField { name: "allowPly", inferred_role: 1 },
    ParticipantField { name: "resevePly", inferred_role: 2 },
    ParticipantField { name: "commanderPly", inferred_role: 3 },
];

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
            let entries =
                kvs.get(field.name).ok_or(ExtractError::MissingField { field: field.name })?;
            for entry in indexed_array_values(entries, field.name)? {
                participants.push(extract_participant(entry, field)?);
            }
        }

        Ok(Section::from_array(participants))
    }
}

fn extract_participant(entry: &Value, field: ParticipantField) -> Result<Value, ExtractError> {
    let entry = entry
        .as_object()
        .ok_or(ExtractError::InvalidFieldType { field: field.name, expected: "object" })?;
    let role = optional_u64_field(entry, "memberType")?.unwrap_or(field.inferred_role);

    Ok(json!({
        "player_name": require_string_field(entry, "Name")?,
        "player_id": require_u64_field(entry, "PlyId")?,
        "power": require_number_field(entry, "Power")?,
        "role": role,
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
        assert_eq!(participants[0]["player_name"], json!("Allowed"));
        assert_eq!(participants[0]["player_id"], json!(30));
        assert_eq!(participants[0]["power"], json!(3000));
        assert_eq!(participants[0]["role"], json!(1));
        assert_eq!(participants[1]["player_name"], json!("Reserve"));
        assert_eq!(participants[2]["player_name"], json!("Commander"));
    }

    #[test]
    fn participants_extractor_infers_missing_member_type_from_source_array() {
        let input = json!({
            "body": {
                "kvs": {
                    "resevePly": [
                        {
                            "Name": "Reserve",
                            "PlyId": 10,
                            "Power": 1000
                        }
                    ],
                    "commanderPly": [
                        {
                            "Name": "Commander",
                            "PlyId": 20,
                            "Power": 2000
                        }
                    ],
                    "allowPly": [
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
        let section = extractor.extract(&input).unwrap();
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 3);
        assert_eq!(participants[0]["role"], json!(1));
        assert_eq!(participants[1]["role"], json!(2));
        assert_eq!(participants[2]["role"], json!(3));
    }

    #[test]
    fn participants_extractor_rejects_invalid_member_type() {
        let input = json!({
            "body": {
                "kvs": {
                    "resevePly": [],
                    "commanderPly": [],
                    "allowPly": [
                        {
                            "Name": "Allowed",
                            "PlyId": 30,
                            "Power": 3000,
                            "memberType": "invalid"
                        }
                    ]
                }
            }
        });

        let extractor = ParticipantsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(
            err,
            ExtractError::InvalidFieldType { field: "memberType", expected: "unsigned integer" }
        ));
    }

    #[test]
    fn roundtrip_participants_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.108726086177768046031.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 30);
        assert_eq!(participants[0]["player_name"], json!("人Øgeday人"));
        assert_eq!(participants[0]["player_id"], json!(40693070));
        assert_eq!(participants[0]["power"], json!(84784300));
        assert_eq!(participants[0]["role"], json!(1));
        assert_eq!(participants[24]["player_name"], json!("ᅠᅠ     ᅠ Swalsh"));
        assert_eq!(participants[25]["player_name"], json!("ˢJianLaars 命"));
    }

    #[test]
    fn roundtrip_participants_infers_missing_member_type_sample_with_commander() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.108518435177768053226.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 14);
        assert_eq!(participants[0]["player_name"], json!("PAC MAN F9"));
        assert_eq!(participants[0]["role"], json!(1));
        assert_eq!(participants[13]["player_name"], json!("Abdul F16"));
        assert_eq!(participants[13]["role"], json!(3));
    }

    #[test]
    fn roundtrip_participants_infers_missing_member_type_allow_only_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.10864788017776806764.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ParticipantsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let participants = section.array().expect("participants");

        assert_eq!(participants.len(), 11);
        assert_eq!(participants[0]["player_name"], json!("ᶠᵖPrettyCat"));
        assert_eq!(participants[0]["role"], json!(1));
        assert_eq!(participants[10]["player_name"], json!("MXA 10"));
        assert_eq!(participants[10]["role"], json!(1));
    }
}
