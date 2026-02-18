//! Body extractor for SystemBarbarianFort mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value};

use crate::content::{
    require_body, require_child_object, require_number_field, require_string_field,
};

/// Extracts position and target details from SystemBarbarianFort mail body.
#[derive(Debug, Default)]
pub struct BodyExtractor;

impl BodyExtractor {
    /// Create a new body extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for BodyExtractor {
    fn section(&self) -> &'static str {
        "body"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let body = require_body(input)?;
        let position = require_child_object(body, "position")?;
        let pos_x = require_number_field(position, "X")?;
        let pos_y = require_number_field(position, "Y")?;
        let target_name = require_string_field(body, "targetName")?;

        let mut section = Section::new();
        section.insert("pos", build_position(pos_x, pos_y));
        section.insert("target_name", Value::String(target_name));
        Ok(section)
    }
}

fn build_position(x: Value, y: Value) -> Value {
    let mut position = Map::new();
    position.insert("x".to_string(), x);
    position.insert("y".to_string(), y);
    Value::Object(position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn body_extractor_reads_fields() {
        let input = json!({
            "body": {
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                },
                "targetName": "Level9"
            }
        });

        let extractor = BodyExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["pos"], json!({ "x": 1.25, "y": 2.75 }));
        assert_eq!(fields["target_name"], json!("Level9"));
    }

    #[test]
    fn body_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "position": {
                    "X": 1.25,
                    "Y": 2.75,
                    "Z": 0
                }
            }
        });
        let extractor = BodyExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_body_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/System/Persistent.Mail.87938122177133895831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BodyExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(
            fields["pos"],
            json!({ "x": 3867.797119140625, "y": 4096.7294921875 })
        );
        assert_eq!(fields["target_name"], json!("Level9"));
    }
}
