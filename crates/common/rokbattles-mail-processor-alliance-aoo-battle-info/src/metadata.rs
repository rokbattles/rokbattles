//! Copies common root metadata into the `metadata` object section.
//!
//! Requires string `id` and `receiver` fields and unsigned integer `time` and
//! `serverId` fields. The SDK renames them to `mail_id`, `mail_receiver`,
//! `mail_time`, and `server_id` without converting timestamp units.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, extract_base_metadata};
use serde_json::Value;

/// Extracts the `metadata` section using the field rules in this module.
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

    use rokbattles_mail_sdk::Extractor;
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
        let section = extractor.extract(&input).unwrap();

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
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_metadata_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.102185425177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("102185425177177256731"));
        assert_eq!(fields["mail_receiver"], json!("player_71738515"));
        assert_eq!(fields["server_id"], json!(2));
        assert_eq!(fields["mail_time"], json!(1771772567839365u64));
    }
}
