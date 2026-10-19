//! Flattens attachment loot into the `rewards` array section.
//!
//! The root `attachments` field and every attachment's `loot` field must be arrays.
//! Each entry requires unsigned `Type`, `SubType`, and `Value`, renamed to `type`,
//! `sub_type`, and `value`. Empty arrays are valid. Attachment order and loot order
//! are preserved, with no deduplication or summing of matching items.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, require_array, require_u64_field};
use serde_json::{Map, Value, json};

/// Extracts loot entries out of SystemKauKaruak attachments.
#[derive(Debug, Default)]
pub struct RewardsExtractor;

impl RewardsExtractor {
    /// Creates a loot extractor.
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
