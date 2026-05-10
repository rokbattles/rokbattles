//! Resources parser for ScoutReport mail.

use mail_processor_sdk::{Extractor, Section, indexed_array_values};
use serde_json::{Value, json};

use crate::content::{ExtractError, require_content, require_u64_field};

/// Pulls scouted resource entries out of ScoutReport mail content.
#[derive(Debug, Default)]
pub struct ResourcesExtractor;

impl ResourcesExtractor {
    /// Creates a resources extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for ResourcesExtractor {
    fn section(&self) -> &'static str {
        "resources"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let resources = content.get("ress").ok_or(ExtractError::MissingField { field: "ress" })?;
        let resources = indexed_array_values(resources, "ress")?;

        let mut entries = Vec::with_capacity(resources.len());
        for resource in resources {
            let resource = resource
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "ress", expected: "object" })?;
            let resource_type = require_u64_field(resource, "type")?;
            let value = require_u64_field(resource, "value")?;
            entries.push(json!({ "type": resource_type, "value": value }));
        }

        Ok(Section::from_array(entries))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn resources_extractor_reads_indexed_resources() {
        let input = json!({
            "body": {
                "content": {
                    "ress": [
                        1,
                        { "type": 1, "value": 10 },
                        2,
                        { "type": 2, "value": 20 }
                    ]
                }
            }
        });
        let extractor = ResourcesExtractor::new();
        let section = extractor.extract(&input).expect("extract resources");
        let resources = section.array().expect("resources array");

        assert_eq!(
            resources,
            [json!({ "type": 1, "value": 10 }), json!({ "type": 2, "value": 20 }),]
        );
    }

    #[test]
    fn resources_extractor_rejects_missing_field() {
        let input = json!({ "body": { "content": {} } });
        let extractor = ResourcesExtractor::new();
        let err = extractor.extract(&input).expect_err("resources should reject missing field");
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_resources_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ResourcesExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let resources = section.array().expect("resources array");

        assert_eq!(resources.len(), 4);
        assert_eq!(resources[0], json!({ "type": 1, "value": 97898017 }));
        assert_eq!(resources[3], json!({ "type": 4, "value": 48852948 }));
    }
}
