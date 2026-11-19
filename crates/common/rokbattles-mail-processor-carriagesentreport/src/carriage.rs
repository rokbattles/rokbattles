//! Extracts the counterparty and transfer details from a carriage report.

use rokbattles_mail_sdk::{
    ExtractError, Extractor, Section, optional_bool_field, optional_number_field_or_zero,
    require_array, require_child_object, require_number_field, require_object,
    require_string_field, require_u64_field,
};
use serde_json::{Value, json};

pub(super) struct CarriageExtractor;

impl Extractor for CarriageExtractor {
    fn section(&self) -> &'static str {
        "carriage"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let body = require_child_object(require_object(input)?, "body")?;
        let content = require_child_object(body, "content")?;

        let resources = content.get("Ress").ok_or(ExtractError::MissingField { field: "Ress" })?;
        let resources = require_array(resources, "Ress")?
            .iter()
            .map(extract_resource)
            .collect::<Result<Vec<_>, _>>()?;

        let target_id = require_u64_field(content, "SendPlayerId")?;
        let target_name = require_string_field(content, "SenderName")?;
        let active = optional_bool_field(content, "Active")?.unwrap_or(false);
        let tax = optional_number_field_or_zero(content, "Tax")?;

        let mut section = Section::new();
        section.insert("resources", Value::Array(resources));
        section.insert("target_id", Value::from(target_id));
        section.insert("target_name", Value::from(target_name));
        section.insert("active", Value::from(active));
        section.insert("tax", tax);

        Ok(section)
    }
}

fn extract_resource(input: &Value) -> Result<Value, ExtractError> {
    let resource = require_object(input)?;

    Ok(json!({
        "type": require_u64_field(resource, "Type")?,
        "value": require_number_field(resource, "Value")?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_resource_order_and_defaults_optional_fields() {
        let section = CarriageExtractor
            .extract(&json!({
                "body": { "content": {
                    "Ress": [{"Type": 3, "Value": 400}, {"Type": 1, "Value": 60}],
                    "SendPlayerId": 51,
                    "SenderName": "Target"
                }}
            }))
            .expect("extract carriage");

        assert_eq!(
            serde_json::to_value(section).expect("serialize carriage"),
            json!({
                "resources": [{"type": 3, "value": 400}, {"type": 1, "value": 60}],
                "target_id": 51,
                "target_name": "Target",
                "active": false,
                "tax": 0
            })
        );
    }

    #[test]
    fn rejects_a_resource_with_an_invalid_amount() {
        let result = CarriageExtractor.extract(&json!({
            "body": { "content": {
                "Ress": [{"Type": 3, "Value": "invalid"}],
                "SendPlayerId": 51,
                "SenderName": "Target"
            }}
        }));

        assert!(matches!(
            result,
            Err(ExtractError::InvalidFieldType { field: "Value", expected: "number" })
        ));
    }
}
