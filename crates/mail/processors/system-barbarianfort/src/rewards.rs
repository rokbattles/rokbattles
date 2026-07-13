//! Rewards parser for SystemBarbarianFort mail.

use mail_sdk::{ExtractError, Extractor, Section, require_array};
use serde_json::{Map, Value, json};

use crate::content::require_u64_field;

/// Pulls reward entries out of SystemBarbarianFort attachments.
#[derive(Debug, Default)]
pub struct RewardsExtractor;

impl RewardsExtractor {
    /// Creates a rewards extractor.
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
            .ok_or(ExtractError::MissingField { field: "attachments" })?;
        let attachments = require_array(attachments, "attachments")?;

        let mut rewards = Vec::new();
        for attachment in attachments {
            let attachment = attachment.as_object().ok_or(ExtractError::InvalidFieldType {
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
    let loot = attachment.get("loot").ok_or(ExtractError::MissingField { field: "loot" })?;
    let loot = require_array(loot, "loot")?;

    for entry in loot {
        let entry = entry
            .as_object()
            .ok_or(ExtractError::InvalidFieldType { field: "loot", expected: "object" })?;
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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn rewards_extractor_reads_fields() {
        let input = json!({
            "attachments": [
                {
                    "loot": [
                        { "Type": 2, "SubType": 26, "Value": 3 },
                        { "Type": 2, "SubType": 65, "Value": 2 }
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
                "sub_type": 26,
                "value": 3
            })
        );
        assert_eq!(
            rewards[1],
            json!({
                "type": 2,
                "sub_type": 65,
                "value": 2
            })
        );
    }

    #[test]
    fn rewards_extractor_reads_all_attachments() {
        let input = json!({
            "attachments": [
                { "loot": [{ "Type": 2, "SubType": 26, "Value": 3 }] },
                { "loot": [{ "Type": 2, "SubType": 65, "Value": 2 }] }
            ]
        });

        let extractor = RewardsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let rewards = section.array().expect("rewards");

        assert_eq!(rewards.len(), 2);
        assert_eq!(rewards[0]["sub_type"], json!(26));
        assert_eq!(rewards[1]["sub_type"], json!(65));
    }

    #[test]
    fn rewards_extractor_rejects_missing_loot() {
        let input = json!({
            "attachments": [
                {
                    "id": 1
                }
            ]
        });
        let extractor = RewardsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_rewards_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../samples/System/Persistent.Mail.87938122177133895831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = RewardsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let rewards = section.array().expect("rewards");

        assert_eq!(rewards.len(), 4);
        assert_eq!(rewards[0], json!({"type": 2, "sub_type": 7006, "value": 64}));
        assert_eq!(rewards[1], json!({"type": 2, "sub_type": 130, "value": 55}));
        assert_eq!(rewards[2], json!({"type": 2, "sub_type": 92, "value": 9}));
        assert_eq!(rewards[3], json!({"type": 2, "sub_type": 182, "value": 1}));
    }
}
