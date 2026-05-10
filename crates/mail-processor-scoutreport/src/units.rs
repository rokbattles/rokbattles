//! Units parser for ScoutReport mail.

use mail_processor_sdk::{Extractor, Section, indexed_array_values};
use serde_json::{Value, json};

use crate::content::{ExtractError, require_child_object, require_content, require_u64_field};

/// Pulls scouted unit entries out of ScoutReport troop content.
#[derive(Debug, Default)]
pub struct UnitsExtractor;

impl UnitsExtractor {
    /// Creates a units extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for UnitsExtractor {
    fn section(&self) -> &'static str {
        "units"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let troop = require_child_object(content, "troop")?;
        let units = troop.get("units").ok_or(ExtractError::MissingField { field: "units" })?;
        let units = indexed_array_values(units, "units")?;

        let mut entries = Vec::with_capacity(units.len());
        for unit in units {
            let unit = unit
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "units", expected: "object" })?;
            let id = require_u64_field(unit, "unitId")?;
            let count = require_u64_field(unit, "unitCount")?;
            let max_count = require_u64_field(unit, "maxCount")?;
            entries.push(json!({ "id": id, "count": count, "max_count": max_count }));
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
    fn units_extractor_reads_troop_units() {
        let input = json!({
            "body": {
                "content": {
                    "troop": {
                        "units": [
                            1,
                            { "unitId": 4, "unitCount": 10, "maxCount": 20 },
                            2,
                            { "unitId": 5, "unitCount": 30, "maxCount": 40 }
                        ]
                    }
                }
            }
        });
        let extractor = UnitsExtractor::new();
        let section = extractor.extract(&input).expect("extract units");
        let units = section.array().expect("units array");

        assert_eq!(
            units,
            [
                json!({ "id": 4, "count": 10, "max_count": 20 }),
                json!({ "id": 5, "count": 30, "max_count": 40 }),
            ]
        );
    }

    #[test]
    fn units_extractor_rejects_missing_field() {
        let input = json!({ "body": { "content": { "troop": {} } } });
        let extractor = UnitsExtractor::new();
        let err = extractor.extract(&input).expect_err("units should reject missing field");
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn roundtrip_units_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ScoutReport/Persistent.Mail.136953280177843782931.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = UnitsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let units = section.array().expect("units array");

        assert_eq!(units.len(), 7);
        assert_eq!(units[0], json!({ "id": 4, "count": 68704, "max_count": 173294 }));
        assert_eq!(units[6], json!({ "id": 16, "count": 666526, "max_count": 666526 }));
    }
}
