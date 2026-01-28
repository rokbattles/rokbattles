//! Summary extractor for Battle mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value, json};

use crate::content::{require_content, require_u64_field};

/// Extracts the sender and opponent battle summaries.
#[derive(Debug, Default)]
pub struct SummaryExtractor;

impl SummaryExtractor {
    /// Create a new summary extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for SummaryExtractor {
    fn section(&self) -> &'static str {
        "summary"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let sender_overview = extract_overview_optional(content.get("SOv"), "SOv")?;
        let opponent_overview = extract_overview_optional(content.get("OOv"), "OOv")?;

        let mut section = Section::new();
        section.insert("sender".to_string(), sender_overview);
        section.insert("opponent".to_string(), opponent_overview);
        Ok(section)
    }
}

/// Read an optional summary payload into the output schema.
fn extract_overview_optional(
    value: Option<&Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match value {
        None | Some(Value::Null) => Ok(null_overview()),
        Some(value) => {
            let overview = value.as_object().ok_or(ExtractError::InvalidFieldType {
                field,
                expected: "object",
            })?;
            extract_overview(overview)
        }
    }
}

/// Normalize a summary overview entry into the output schema.
fn extract_overview(overview: &Map<String, Value>) -> Result<Value, ExtractError> {
    let kill_points = require_u64_field(overview, "KillScore")?;
    let dead = require_u64_field(overview, "Dead")?;
    let severely_wounded = require_u64_field(overview, "BadHurt")?;
    let slightly_wounded = require_u64_field(overview, "Hurt")?;
    let remaining = require_u64_field(overview, "Cnt")?;
    let troop_units = require_u64_field(overview, "Max")?;

    Ok(json!({
        "kill_points": kill_points,
        "dead": dead,
        "severely_wounded": severely_wounded,
        "slightly_wounded": slightly_wounded,
        "remaining": remaining,
        "troop_units": troop_units,
    }))
}

/// Build a null-filled overview when the payload is missing.
fn null_overview() -> Value {
    json!({
        "kill_points": Value::Null,
        "dead": Value::Null,
        "severely_wounded": Value::Null,
        "slightly_wounded": Value::Null,
        "remaining": Value::Null,
        "troop_units": Value::Null,
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
    fn summary_extractor_reads_overviews() {
        let input = json!({
            "body": {
                "content": {
                    "SOv": {
                        "KillScore": 10,
                        "Dead": 1,
                        "BadHurt": 2,
                        "Hurt": 3,
                        "Cnt": 4,
                        "Max": 5
                    },
                    "OOv": {
                        "KillScore": 20,
                        "Dead": 6,
                        "BadHurt": 7,
                        "Hurt": 8,
                        "Cnt": 9,
                        "Max": 10
                    }
                }
            }
        });
        let extractor = SummaryExtractor::new();
        let section = extractor.extract(&input).expect("summary section");

        let fields = section.fields();
        assert_eq!(
            fields["sender"],
            json!({
                "kill_points": 10,
                "dead": 1,
                "severely_wounded": 2,
                "slightly_wounded": 3,
                "remaining": 4,
                "troop_units": 5
            })
        );
        assert_eq!(
            fields["opponent"],
            json!({
                "kill_points": 20,
                "dead": 6,
                "severely_wounded": 7,
                "slightly_wounded": 8,
                "remaining": 9,
                "troop_units": 10
            })
        );
    }

    #[test]
    fn summary_extractor_rejects_missing_sender_overview() {
        let input = json!({
            "body": {
                "content": {
                    "OOv": {
                        "KillScore": 1,
                        "Dead": 2,
                        "BadHurt": 3,
                        "Hurt": 4,
                        "Cnt": 5,
                        "Max": 6
                    }
                }
            }
        });
        let extractor = SummaryExtractor::new();
        let section = extractor.extract(&input).expect("summary section");
        let fields = section.fields();
        assert_eq!(fields["sender"], null_overview());
        assert_eq!(fields["opponent"]["kill_points"], json!(1));
    }

    #[test]
    fn summary_extractor_rejects_missing_opponent_overview() {
        let input = json!({
            "body": {
                "content": {
                    "SOv": {
                        "KillScore": 1,
                        "Dead": 2,
                        "BadHurt": 3,
                        "Hurt": 4,
                        "Cnt": 5,
                        "Max": 6
                    }
                }
            }
        });
        let extractor = SummaryExtractor::new();
        let section = extractor.extract(&input).expect("summary section");
        let fields = section.fields();
        assert_eq!(fields["sender"]["kill_points"], json!(1));
        assert_eq!(fields["opponent"], null_overview());
    }

    #[test]
    fn roundtrip_summary_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.10224136175529255431.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = SummaryExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(
            fields["sender"],
            json!({
                "kill_points": 307090,
                "dead": 0,
                "severely_wounded": 14528,
                "slightly_wounded": 85754,
                "remaining": 129718,
                "troop_units": 230000
            })
        );
        assert_eq!(
            fields["opponent"],
            json!({
                "kill_points": 290560,
                "dead": 0,
                "severely_wounded": 30709,
                "slightly_wounded": 166508,
                "remaining": 17659,
                "troop_units": 418491
            })
        );
    }
}
