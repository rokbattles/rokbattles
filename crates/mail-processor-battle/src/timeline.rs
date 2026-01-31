//! Timeline extractor for Battle mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, indexed_array_values, require_u64};
use serde_json::{Map, Value, json};

use crate::content::{
    require_child_object, require_content, require_string_field, require_u64_field,
};
use crate::player::parse_avatar;

/// Extracts timeline snapshots from Battle mail.
#[derive(Debug, Default)]
pub struct TimelineExtractor;

impl TimelineExtractor {
    /// Create a new timeline extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for TimelineExtractor {
    fn section(&self) -> &'static str {
        "timeline"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let start_timestamp = require_u64_field(content, "Bts")?;
        let end_timestamp = require_u64_field(content, "Ets")?;
        let start_tick = require_u64_field(content, "Btk")?;
        let samples_value = content
            .get("Samples")
            .ok_or(ExtractError::MissingField { field: "Samples" })?;
        let samples = indexed_array_values(samples_value, "Samples")?;

        let mut entries = Vec::with_capacity(samples.len());
        for sample in samples {
            let tick = require_u64(sample, "T")?;
            let count = require_u64(sample, "Cnt")?;
            entries.push(json!({ "tick": tick, "count": count }));
        }

        // Event type (Et) mappings:
        // - 18: reinforcements join
        // - 26: reinforcements leave (Cnt may be omitted when march count hits 0)
        let events_value = content
            .get("Events")
            .ok_or(ExtractError::MissingField { field: "Events" })?;
        let events = indexed_array_values(events_value, "Events")?;
        let mut event_entries = Vec::with_capacity(events.len());
        for event in events {
            let event_map = event.as_object().ok_or(ExtractError::InvalidFieldType {
                field: "Events",
                expected: "object",
            })?;
            let tick = require_u64(event, "T")?;
            let event_type = require_u64(event, "Et")?;
            let assist_units = require_child_object(event_map, "AssistUnits")?;
            let player_id = require_signed_id_field(assist_units, "PId")?;
            let player_name = require_string_field(assist_units, "PName")?;
            let count = optional_u64_field(assist_units, "Cnt")?;
            let event_id = optional_u64_field(assist_units, "TId")?;
            let (avatar_url, frame_url) = parse_avatar(assist_units)?;
            let primary_id = optional_u64_field(assist_units, "HId")?;
            let primary_level = optional_u64_field(assist_units, "HLv")?;
            let secondary_id = optional_u64_field(assist_units, "HId2")?;
            let secondary_level = optional_u64_field(assist_units, "HLv2")?;
            event_entries.push(json!({
                "tick": tick,
                "type": event_type,
                "event_id": event_id,
                "player_id": player_id,
                "player_name": player_name,
                "count": count.map(Value::from).unwrap_or(Value::Null),
                "avatar_url": avatar_url,
                "frame_url": frame_url,
                "commanders": {
                    "primary": { "id": primary_id, "level": primary_level },
                    "secondary": { "id": secondary_id, "level": secondary_level },
                },
            }));
        }

        let mut section = Section::new();
        section.insert("start_timestamp", Value::from(start_timestamp));
        section.insert("end_timestamp", Value::from(end_timestamp));
        section.insert("start_tick", Value::from(start_tick));
        section.insert("sampling", Value::Array(entries));
        section.insert("events", Value::Array(event_entries));
        Ok(section)
    }
}

/// Require a numeric identifier that can be either signed or unsigned.
fn require_signed_id_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<i64, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    if let Some(id) = value.as_i64() {
        return Ok(id);
    }
    if let Some(id) = value.as_u64() {
        return i64::try_from(id).map_err(|_| ExtractError::InvalidFieldType {
            field,
            expected: "signed 64-bit integer",
        });
    }
    Err(ExtractError::InvalidFieldType {
        field,
        expected: "integer",
    })
}

/// Read an optional unsigned integer field from a JSON object.
fn optional_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, ExtractError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or(ExtractError::InvalidFieldType {
                field,
                expected: "unsigned integer",
            }),
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
    fn timeline_extractor_reads_samples() {
        let input = json!({
            "body": {
                "content": {
                    "Bts": 10,
                    "Ets": 20,
                    "Btk": 5,
                    "Events": [],
                    "Samples": [
                        1,
                        { "Cnt": 10, "T": 100 },
                        2,
                        { "Cnt": 20, "T": 200 }
                    ]
                }
            }
        });
        let extractor = TimelineExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["start_timestamp"], json!(10));
        assert_eq!(fields["end_timestamp"], json!(20));
        assert_eq!(fields["start_tick"], json!(5));
        let samples = fields["sampling"].as_array().expect("sampling array");
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], json!({ "tick": 100, "count": 10 }));
        assert_eq!(samples[1], json!({ "tick": 200, "count": 20 }));
    }

    #[test]
    fn timeline_extractor_reads_events() {
        let input = json!({
            "body": {
                "content": {
                    "Bts": 10,
                    "Ets": 20,
                    "Btk": 5,
                    "Samples": [],
                    "Events": [
                        1,
                        {
                            "T": 7,
                            "Et": 18,
                            "AssistUnits": {
                            "PId": 42,
                            "PName": "Alpha",
                            "Cnt": 99,
                            "TId": 1234,
                            "Avatar": "null",
                            "HId": 10,
                            "HLv": 20,
                                "HId2": 11,
                                "HLv2": 21
                            }
                        }
                    ]
                }
            }
        });
        let extractor = TimelineExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        let events = fields["events"].as_array().expect("events array");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            json!({
                "tick": 7,
                "type": 18,
                "event_id": 1234,
                "player_id": 42,
                "player_name": "Alpha",
                "count": 99,
                "avatar_url": Value::Null,
                "frame_url": Value::Null,
                "commanders": {
                    "primary": { "id": 10, "level": 20 },
                    "secondary": { "id": 11, "level": 21 },
                }
            })
        );
    }

    #[test]
    fn timeline_extractor_allows_event_without_count() {
        let input = json!({
            "body": {
                "content": {
                    "Bts": 10,
                    "Ets": 20,
                    "Btk": 5,
                    "Samples": [],
                    "Events": [
                        1,
                        {
                            "T": 7,
                            "Et": 18,
                            "AssistUnits": {
                                "PId": 42,
                                "PName": "Alpha",
                                "Avatar": "null",
                                "HId": 10,
                                "HLv": 20,
                                "HId2": 11,
                                "HLv2": 21
                            }
                        }
                    ]
                }
            }
        });
        let extractor = TimelineExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        let events = fields["events"].as_array().expect("events array");
        assert_eq!(events.len(), 1);
        assert!(events[0]["count"].is_null());
    }

    #[test]
    fn timeline_extractor_rejects_missing_samples() {
        let input = json!({
            "body": {
                "content": {
                    "Bts": 10,
                    "Ets": 20,
                    "Btk": 5
                }
            }
        });
        let extractor = TimelineExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_timeline_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = TimelineExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["start_timestamp"], json!(1755294123));
        assert_eq!(fields["end_timestamp"], json!(1755294123));
        assert_eq!(fields["start_tick"], json!(312472));
        let samples = fields["sampling"].as_array().expect("sampling array");
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], json!({ "tick": 312472, "count": 224250 }));
        assert_eq!(samples[1], json!({ "tick": 312472, "count": 224250 }));
        let events = fields["events"].as_array().expect("events array");
        assert!(events.is_empty());
    }

    #[test]
    fn roundtrip_timeline_extracts_large_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.8421200117137313126.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = TimelineExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        let events = fields["events"].as_array().expect("events array");
        assert!(!events.is_empty());
        assert!(events.iter().any(|event| event["count"].is_null()));
    }
}
