//! Metadata extractor for Battle mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_string, require_u64};
use serde_json::Value;

use crate::content::{require_content, require_string_field};

/// Extracts top-level metadata fields from a Battle mail.
#[derive(Debug, Default)]
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Create a new metadata extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for MetadataExtractor {
    fn section(&self) -> &'static str {
        "metadata"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let mail_id = require_string(input, "id")?;
        let mail_time = require_u64(input, "time")?;
        let mail_receiver = require_string(input, "receiver")?;
        let server_id = require_u64(input, "serverId")?;
        let content = require_content(input)?;
        let mail_role = require_string_field(content, "Role")?;

        let mut section = Section::new();
        section.insert("mail_id", Value::String(mail_id));
        section.insert("mail_time", Value::from(mail_time));
        section.insert("mail_receiver", Value::String(mail_receiver));
        section.insert("server_id", Value::from(server_id));
        section.insert("mail_role", Value::String(mail_role));
        Ok(section)
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
    fn metadata_extractor_reads_fields() {
        let input = json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": {
                "content": {
                    "Role": "gsmp"
                }
            }
        });
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("mail-1"));
        assert_eq!(fields["mail_time"], json!(1234));
        assert_eq!(fields["mail_receiver"], json!("player-1"));
        assert_eq!(fields["server_id"], json!(55));
        assert_eq!(fields["mail_role"], json!("gsmp"));
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
            .join("../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = MetadataExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(fields["mail_id"], json!("1002579517552941234"));
        assert_eq!(fields["mail_receiver"], json!("player_110176153"));
        assert_eq!(fields["server_id"], json!(1804));
        assert_eq!(fields["mail_time"], json!(1755294123041275u64));
        assert_eq!(fields["mail_role"], json!("gsmp"));
    }
}
