//! Metadata parser for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, extract_base_metadata};
use serde_json::Value;

/// Pulls top-level metadata out of an AllianceAOOIndividualResults mail.
#[derive(Debug, Default)]
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Creates a metadata extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for MetadataExtractor {
    fn section(&self) -> &'static str {
        "metadata"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let metadata = extract_base_metadata(input)?;
        let custom = matches!(extract_body_type(input), Some(14 | 15));

        let mut section = metadata.into_section();
        section.insert("custom", Value::Bool(custom));
        Ok(section)
    }
}

fn extract_body_type(input: &Value) -> Option<u64> {
    input
        .get("body")
        .and_then(Value::as_object)
        .and_then(|body| body.get("type"))
        .and_then(value_as_u64)
}

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn metadata_extractor_reads_fields() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": { "type": 62 }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("mail-1"));
        assert_eq!(fields["mail_time"], json!(1234));
        assert_eq!(fields["mail_receiver"], json!("player-1"));
        assert_eq!(fields["server_id"], json!(55));
        assert_eq!(fields["custom"], json!(false));
    }

    #[test]
    fn metadata_extractor_rejects_missing_field() {
        let input = json!({ "id": "mail-1" });
        let extractor = MetadataExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_metadata_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("102185429177177256731"));
        assert_eq!(fields["mail_receiver"], json!("player_71738515"));
        assert_eq!(fields["server_id"], json!(2));
        assert_eq!(fields["mail_time"], json!(1771772567843913u64));
        assert_eq!(fields["custom"], json!(false));
    }

    #[test]
    fn metadata_extractor_sets_custom_true_for_custom_type() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": { "type": 15 }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["custom"], json!(true));
    }
}
