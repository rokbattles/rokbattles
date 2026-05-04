//! Overview parser for AllianceAOORegistration mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_u64_field};
use serde_json::Value;

use crate::content::require_body_kvs;

/// Pulls registration timing fields from `body.kvs`.
#[derive(Debug, Default)]
pub struct OverviewExtractor;

impl OverviewExtractor {
    /// Creates an overview extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for OverviewExtractor {
    fn section(&self) -> &'static str {
        "overview"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let kvs = require_body_kvs(input)?;
        let mut section = Section::new();
        section.insert("start_time", Value::from(require_u64_field(kvs, "startTime")?));
        Ok(section)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn overview_extractor_reads_start_time() {
        let input = json!({
            "body": {
                "kvs": {
                    "startTime": 1234
                }
            }
        });

        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["start_time"], json!(1234));
    }

    #[test]
    fn overview_extractor_rejects_missing_start_time() {
        let input = json!({
            "body": {
                "kvs": {}
            }
        });

        let extractor = OverviewExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "startTime" }));
    }

    #[test]
    fn roundtrip_overview_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.1087260861777680.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OverviewExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["start_time"], json!(1777816800000u64));
    }
}
