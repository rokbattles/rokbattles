//! Rewards extractor for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, indexed_array_values};
use serde_json::{Map, Value, json};

/// Extracts reward entries from AllianceAOOIndividualResults attachments.
#[derive(Debug, Default)]
pub struct RewardsExtractor;

impl RewardsExtractor {
    /// Create a new rewards extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for RewardsExtractor {
    fn section(&self) -> &'static str {
        "rewards"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let attachments = input
            .as_object()
            .and_then(|root| root.get("attachments"))
            .ok_or(ExtractError::MissingField {
                field: "attachments",
            })?;
        let attachments = indexed_array_values(attachments, "attachments")?;

        let mut rewards = Vec::new();
        for attachment in attachments {
            let attachment = attachment
                .as_object()
                .ok_or(ExtractError::InvalidFieldType {
                    field: "attachments",
                    expected: "object",
                })?;
            extract_rewards(attachment, &mut rewards)?;
        }

        Ok(Section::from_array(rewards))
    }
}

fn extract_rewards(
    attachment: &Map<String, Value>,
    rewards: &mut Vec<Value>,
) -> Result<(), ExtractError> {
    let loot = attachment
        .get("loot")
        .ok_or(ExtractError::MissingField { field: "loot" })?;
    let loot = indexed_array_values(loot, "loot")?;

    for entry in loot {
        let entry = entry.as_object().ok_or(ExtractError::InvalidFieldType {
            field: "loot",
            expected: "object",
        })?;
        let reward_type = require_u64_field(entry, "Type")?;
        let sub_type = require_u64_field(entry, "SubType")?;
        let value = require_u64_field(entry, "Value")?;

        rewards.push(json!({
            "type": reward_type,
            "sub_type": sub_type,
            "value": value,
        }));
    }

    Ok(())
}

fn require_u64_field(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_u64().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "unsigned integer",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn rewards_extractor_reads_fields() {
        let input = json!({
            "attachments": [
                1,
                {
                    "loot": [
                        1,
                        { "Type": 2, "SubType": 30, "Value": 1 },
                        2,
                        { "Type": 2, "SubType": 44, "Value": 2 }
                    ]
                }
            ]
        });

        let extractor = RewardsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let rewards = section.array().expect("rewards");

        assert_eq!(rewards.len(), 2);
        assert_eq!(
            rewards[0],
            json!({
                "type": 2,
                "sub_type": 30,
                "value": 1
            })
        );
        assert_eq!(
            rewards[1],
            json!({
                "type": 2,
                "sub_type": 44,
                "value": 2
            })
        );
    }

    #[test]
    fn rewards_extractor_rejects_missing_attachments() {
        let input = json!({ "id": "mail-1" });
        let extractor = RewardsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(
            err,
            ExtractError::MissingField {
                field: "attachments"
            }
        ));
    }

    #[test]
    fn rewards_extractor_rejects_missing_loot() {
        let input = json!({
            "attachments": [
                1,
                {
                    "id": 1
                }
            ]
        });
        let extractor = RewardsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "loot" }));
    }

    #[test]
    fn roundtrip_rewards_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = RewardsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let rewards = section.array().expect("rewards");

        assert_eq!(rewards.len(), 6);
        assert_eq!(rewards[0], json!({"type": 2, "sub_type": 32, "value": 1}));
        assert_eq!(rewards[5], json!({"type": 2, "sub_type": 188, "value": 1}));
    }
}
