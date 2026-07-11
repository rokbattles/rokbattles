//! Metadata parser for ScoutReport mail.

use mail_sdk::{ExtractError, Extractor, Section, extract_base_metadata};
use serde_json::Value;

/// Pulls top-level metadata out of a ScoutReport mail.
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
        Ok(extract_base_metadata(input)?.into_section())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn metadata_extractor_reads_fields() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).expect("extract metadata");

        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("mail-1"));
        assert_eq!(fields["mail_time"], json!(1234));
        assert_eq!(fields["mail_receiver"], json!("player-1"));
        assert_eq!(fields["server_id"], json!(55));
    }

    #[test]
    fn metadata_extractor_rejects_missing_field() {
        let input = json!({ "id": "mail-1" });
        let extractor = MetadataExtractor::new();
        let err = extractor.extract(&input).expect_err("metadata should reject missing fields");
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_metadata_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("136953280177843782931"));
        assert_eq!(fields["mail_receiver"], json!("player_71738515"));
        assert_eq!(fields["server_id"], json!(1804));
        assert_eq!(fields["mail_time"], json!(1778437829182528u64));
    }
}
