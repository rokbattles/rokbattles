//! Loot parser for SystemKaharTreasure mail.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, require_array, require_u64_field};
use serde_json::{Map, Value, json};

/// Pulls loot entries out of SystemKaharTreasure attachments.
#[derive(Debug, Default)]
pub struct LootExtractor;

impl LootExtractor {
    /// Creates a loot extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for LootExtractor {
    fn section(&self) -> &'static str {
        "loot"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let attachments = input
            .as_object()
            .and_then(|root| root.get("attachments"))
            .ok_or(ExtractError::MissingField { field: "attachments" })?;
        let attachments = require_array(attachments, "attachments")?;

        let mut loot = Vec::new();
        for attachment in attachments {
            let attachment = attachment.as_object().ok_or(ExtractError::InvalidFieldType {
                field: "attachments",
                expected: "object",
            })?;
            extract_loot(attachment, &mut loot)?;
        }

        Ok(Section::from_array(loot))
    }
}

fn extract_loot(
    attachment: &Map<String, Value>,
    output: &mut Vec<Value>,
) -> Result<(), ExtractError> {
    let loot = attachment.get("loot").ok_or(ExtractError::MissingField { field: "loot" })?;
    let loot = require_array(loot, "loot")?;

    for entry in loot {
        let entry = entry
            .as_object()
            .ok_or(ExtractError::InvalidFieldType { field: "loot", expected: "object" })?;
        let loot_type = require_u64_field(entry, "Type")?;
        let sub_type = require_u64_field(entry, "SubType")?;
        let value = require_u64_field(entry, "Value")?;

        output.push(json!({
            "type": loot_type,
            "sub_type": sub_type,
            "value": value,
        }));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rokbattles_mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn loot_extractor_reads_fields() {
        let input = json!({
            "attachments": [
                {
                    "loot": [
                        { "Type": 1, "SubType": 9, "Value": 45000 },
                        { "Type": 2, "SubType": 147, "Value": 5 }
                    ]
                }
            ]
        });

        let extractor = LootExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let loot = section.array().expect("loot");

        assert_eq!(loot.len(), 2);
        assert_eq!(loot[0], json!({"type": 1, "sub_type": 9, "value": 45000}));
        assert_eq!(loot[1], json!({"type": 2, "sub_type": 147, "value": 5}));
    }

    #[test]
    fn loot_extractor_reads_all_attachments() {
        let input = json!({
            "attachments": [
                { "loot": [{ "Type": 1, "SubType": 9, "Value": 45000 }] },
                { "loot": [{ "Type": 2, "SubType": 147, "Value": 5 }] }
            ]
        });

        let extractor = LootExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let loot = section.array().expect("loot");

        assert_eq!(loot.len(), 2);
        assert_eq!(loot[0]["sub_type"], json!(9));
        assert_eq!(loot[1]["sub_type"], json!(147));
    }

    #[test]
    fn loot_extractor_rejects_missing_loot() {
        let input = json!({
            "attachments": [
                {
                    "id": 1
                }
            ]
        });
        let extractor = LootExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "loot" }));
    }

    #[test]
    fn roundtrip_loot_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/System/Persistent.Mail.22165348178347040031.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = LootExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let loot = section.array().expect("loot");

        assert_eq!(loot.len(), 5);
        assert_eq!(loot[0], json!({"type": 1, "sub_type": 9, "value": 45000}));
        assert_eq!(loot[1], json!({"type": 2, "sub_type": 147, "value": 5}));
        assert_eq!(loot[2], json!({"type": 2, "sub_type": 10, "value": 1}));
        assert_eq!(loot[3], json!({"type": 2, "sub_type": 109, "value": 3}));
        assert_eq!(loot[4], json!({"type": 2, "sub_type": 94, "value": 1}));
    }
}
