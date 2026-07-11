//! RSS parser for RSS mail content.

use mail_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value};

use crate::content::{
    optional_number_field_or_zero, require_child_object, require_content, require_number_field,
};

/// Pulls resource report fields from `body.content`.
#[derive(Debug, Default)]
pub struct RssExtractor;

impl RssExtractor {
    /// Creates an RSS extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for RssExtractor {
    fn section(&self) -> &'static str {
        "rss"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let pos = require_child_object(content, "Pos")?;

        let rss_type = require_number_field(content, "ResType")?;
        let rss_value = require_number_field(content, "ResValue")?;
        let rss_bonus = require_number_field(content, "talentAdd")?;
        let time = require_number_field(content, "Time")?;
        let level = require_number_field(content, "Level")?;
        let crystals_gain = optional_number_field_or_zero(content, "ResCollectCrystal")?;
        let pos_x = require_number_field(pos, "X")?;
        let pos_y = require_number_field(pos, "Y")?;

        let mut section = Section::new();
        section.insert("rss_type", rss_type);
        section.insert("rss_value", rss_value);
        section.insert("rss_bonus", rss_bonus);
        section.insert("time", time);
        section.insert("level", level);
        section.insert("pos", build_position(pos_x, pos_y));
        section.insert("crystals_gain", crystals_gain);
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
    use std::{fs, path::PathBuf};

    use mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn rss_extractor_reads_fields() {
        let input = json!({
            "body": {
                "content": {
                    "ResType": 2,
                    "ResValue": 742.5,
                    "talentAdd": 148,
                    "Time": 1772127667,
                    "Level": 8,
                    "ResCollectCrystal": 0,
                    "Pos": {
                        "X": 3784.925537109375,
                        "Y": 3969.92431640625
                    }
                }
            }
        });

        let extractor = RssExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["rss_type"], json!(2));
        assert_eq!(fields["rss_value"], json!(742.5));
        assert_eq!(fields["rss_bonus"], json!(148));
        assert_eq!(fields["time"], json!(1772127667));
        assert_eq!(fields["level"], json!(8));
        assert_eq!(fields["pos"], json!({ "x": 3784.925537109375, "y": 3969.92431640625 }));
        assert_eq!(fields["crystals_gain"], json!(0));
    }

    #[test]
    fn rss_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "content": {
                    "ResType": 2,
                    "talentAdd": 148,
                    "Time": 1772127667,
                    "Level": 8,
                    "ResCollectCrystal": 0,
                    "Pos": {
                        "X": 3784.925537109375,
                        "Y": 3969.92431640625
                    }
                }
            }
        });

        let extractor = RssExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "ResValue" }));
    }

    #[test]
    fn roundtrip_rss_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Rss/Persistent.Mail.113164877177212776431.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = RssExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["rss_type"], json!(1));
        assert_eq!(fields["rss_value"], json!(4104.32));
        assert_eq!(fields["rss_bonus"], json!(232));
        assert_eq!(fields["time"], json!(1772127764));
        assert_eq!(fields["level"], json!(6));
        assert_eq!(fields["pos"], json!({ "x": 3804.365966796875, "y": 3906.101318359375 }));
        assert_eq!(fields["crystals_gain"], json!(0));
    }

    #[test]
    fn rss_extractor_defaults_missing_crystals_gain_to_zero() {
        let input = json!({
            "body": {
                "content": {
                    "ResType": 3,
                    "ResValue": 260,
                    "talentAdd": 0,
                    "Time": 1649934053,
                    "Level": 4,
                    "Pos": {
                        "X": 3215.610107421875,
                        "Y": 4566.96923828125
                    }
                }
            }
        });

        let extractor = RssExtractor::new();
        let section = extractor.extract(&input).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["crystals_gain"], json!(0));
    }

    #[test]
    fn roundtrip_rss_extracts_sample_without_crystals_gain() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Rss/Persistent.Mail.118801516499340535.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = RssExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["rss_type"], json!(3));
        assert_eq!(fields["rss_value"], json!(260));
        assert_eq!(fields["rss_bonus"], json!(0));
        assert_eq!(fields["time"], json!(1649934053));
        assert_eq!(fields["level"], json!(4));
        assert_eq!(fields["pos"], json!({ "x": 3215.610107421875, "y": 4566.96923828125 }));
        assert_eq!(fields["crystals_gain"], json!(0));
    }
}
