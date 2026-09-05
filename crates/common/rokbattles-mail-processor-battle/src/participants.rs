//! Converts keyed rally or garrison participants into rows sorted by signed ID.
//!
//! Missing, null, and empty-array tables all become empty participant arrays.
//! Other inputs must be objects keyed by integers. Row `PId` stays signed;
//! missing alliance abbreviations become empty strings and missing commander
//! fields become null. The table key and player ID remain separate output fields.

use rokbattles_mail_sdk::{
    ExtractError, optional_string_field, optional_u64_field, require_i64_field,
};
use serde_json::{Map, Value, json};

use crate::content::require_string_field;

/// Reads participant objects from the requested field.
pub(crate) fn extract_participants(
    container: &Map<String, Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    let value = match container.get(field) {
        None | Some(Value::Null) => return Ok(Value::Array(Vec::new())),
        Some(value) => value,
    };
    let participants = match value {
        Value::Object(participants) => participants,
        Value::Array(items) if items.is_empty() => return Ok(Value::Array(Vec::new())),
        _ => {
            return Err(ExtractError::InvalidFieldType { field, expected: "object" });
        }
    };

    let mut entries = Vec::with_capacity(participants.len());
    for (participant_id, participant) in participants {
        let participant = participant
            .as_object()
            .ok_or(ExtractError::InvalidFieldType { field, expected: "object" })?;
        let participant_id = parse_participant_id(participant_id, field)?;
        let player_id = require_i64_field(participant, "PId")?;
        let player_name = require_string_field(participant, "PName")?;
        // Keep the legacy empty-string representation for an unreported alliance tag.
        let alliance_abbr = optional_string_field(participant, "Abbr")?.unwrap_or_default();
        let primary_id = optional_u64_field(participant, "HId")?;
        let primary_level = optional_u64_field(participant, "HLv")?;
        let secondary_id = optional_u64_field(participant, "HId2")?;
        let secondary_level = optional_u64_field(participant, "HLv2")?;
        entries.push(json!({
            "participant_id": participant_id,
            "player_id": player_id,
            "player_name": player_name,
            "alliance": { "abbreviation": alliance_abbr },
            "commanders": {
                "primary": { "id": primary_id, "level": primary_level },
                "secondary": { "id": secondary_id, "level": secondary_level },
            },
        }));
    }

    // Signed numeric ordering preserves negative participant IDs and avoids string ordering.
    entries.sort_by_key(|entry| entry["participant_id"].as_i64().unwrap_or_default());
    Ok(Value::Array(entries))
}

fn parse_participant_id(participant_id: &str, field: &'static str) -> Result<i64, ExtractError> {
    participant_id
        .parse::<i64>()
        .map_err(|_| ExtractError::InvalidFieldType { field, expected: "numeric object key" })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn extract_participants_reads_entries() {
        let input = json!({
            "STs": {
                "-2": {
                    "PId": 100,
                    "PName": "Alpha",
                    "Abbr": "AA",
                    "HId": 10,
                    "HLv": 20,
                    "HId2": 11,
                    "HLv2": 21
                },
                "3": {
                    "PId": 101,
                    "PName": "Beta",
                    "Abbr": "BB",
                    "HId": 12,
                    "HLv": 22,
                    "HId2": 13,
                    "HLv2": 23
                }
            }
        });

        let participants = extract_participants(input.as_object().unwrap(), "STs").unwrap();
        assert_eq!(
            participants,
            json!([
                {
                    "participant_id": -2,
                    "player_id": 100,
                    "player_name": "Alpha",
                    "alliance": { "abbreviation": "AA" },
                    "commanders": {
                        "primary": { "id": 10, "level": 20 },
                        "secondary": { "id": 11, "level": 21 },
                    }
                },
                {
                    "participant_id": 3,
                    "player_id": 101,
                    "player_name": "Beta",
                    "alliance": { "abbreviation": "BB" },
                    "commanders": {
                        "primary": { "id": 12, "level": 22 },
                        "secondary": { "id": 13, "level": 23 },
                    }
                }
            ])
        );
    }

    #[test]
    fn extract_participants_defaults_missing_abbr() {
        let input = json!({
            "STs": {
                "1": {
                    "PId": 100,
                    "PName": "Alpha",
                    "HId": 10,
                    "HLv": 20,
                    "HId2": 11,
                    "HLv2": 21
                }
            }
        });

        let participants = extract_participants(input.as_object().unwrap(), "STs").unwrap();
        assert_eq!(
            participants,
            json!([
                {
                    "participant_id": 1,
                    "player_id": 100,
                    "player_name": "Alpha",
                    "alliance": { "abbreviation": "" },
                    "commanders": {
                        "primary": { "id": 10, "level": 20 },
                        "secondary": { "id": 11, "level": 21 },
                    }
                }
            ])
        );
    }

    #[test]
    fn extract_participants_allows_missing_field() {
        let input = json!({});
        let participants = extract_participants(input.as_object().unwrap(), "STs").unwrap();
        assert_eq!(participants, Value::Array(Vec::new()));
    }

    #[test]
    fn extract_participants_allows_empty_array() {
        let input = json!({ "OTs": [] });
        let participants = extract_participants(input.as_object().unwrap(), "OTs").unwrap();
        assert_eq!(participants, Value::Array(Vec::new()));
    }
}
