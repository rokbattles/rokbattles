//! Results extractor for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_object};
use serde_json::{Map, Value};

/// Extracts high-level individual match results from `body.kvs.FightReport`.
#[derive(Debug, Default)]
pub struct ResultsExtractor;

impl ResultsExtractor {
    /// Create a new results extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for ResultsExtractor {
    fn section(&self) -> &'static str {
        "results"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let root = require_object(input)?;
        let body = require_child_object(root, "body")?;
        let kvs = require_child_object(body, "kvs")?;
        let fight_report = require_child_object(kvs, "FightReport")?;

        let total_score = require_u64_field(fight_report, "TotalScore")?;
        let win_rate = require_u64_field(fight_report, "WinRate")?;
        let battles_win = require_u64_field(fight_report, "FightWin")?;
        let battles_lose = require_u64_field_any(fight_report, &["FightLose", "FghtLose"])?;
        let severely_wounded = require_u64_field(fight_report, "BeKilled")?;
        let kills = require_u64_field(fight_report, "Killed")?;
        let kill_score = require_u64_field(fight_report, "KillScore")?;
        let flag_score = require_u64_field(fight_report, "FlagScore")?;
        let building_score = require_u64_field(fight_report, "BuildingScore")?;
        let gather_score = require_u64_field(fight_report, "GatherScore")?;
        let healing_score = require_u64_field(fight_report, "HealingScore")?;
        let units_healed = require_u64_field(fight_report, "HealingCnt")?;
        let flag_count = require_u64_field(fight_report, "FlagCnt")?;
        let teleports = require_u64_field(fight_report, "RelocateCnt")?;
        let speedups = require_u64_field(fight_report, "SpeedUpTime")?;
        let structures = require_u64_field(fight_report, "OccupyCnt")?;

        let mut section = Section::new();
        // Individual Points
        section.insert("total_score", Value::from(total_score));
        // Win Percentage
        section.insert("win_rate", Value::from(win_rate));
        section.insert("battles_win", Value::from(battles_win));
        section.insert("battles_lose", Value::from(battles_lose));
        section.insert("severely_wounded", Value::from(severely_wounded));
        section.insert("kills", Value::from(kills));
        section.insert("kill_score", Value::from(kill_score));
        // Ark of Osiris Score
        section.insert("flag_score", Value::from(flag_score));
        // Occupation Score
        section.insert("building_score", Value::from(building_score));
        // Provisions Score
        section.insert("gather_score", Value::from(gather_score));
        section.insert("healing_score", Value::from(healing_score));
        section.insert("units_healed", Value::from(units_healed));
        // Arks Captured
        section.insert("flag_count", Value::from(flag_count));
        // teleports used
        section.insert("teleports", Value::from(teleports));
        // minutes used
        section.insert("speedups", Value::from(speedups));
        // structures reinforced
        section.insert("structures", Value::from(structures));
        Ok(section)
    }
}

fn require_child_object<'a>(
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ExtractError> {
    let value = object
        .get(field)
        .ok_or(ExtractError::MissingField { field })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType {
        field,
        expected: "object",
    })
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

fn require_u64_field_any(
    object: &Map<String, Value>,
    fields: &[&'static str],
) -> Result<u64, ExtractError> {
    for field in fields {
        if let Some(value) = object.get(*field) {
            return value.as_u64().ok_or(ExtractError::InvalidFieldType {
                field,
                expected: "unsigned integer",
            });
        }
    }

    Err(ExtractError::MissingField { field: fields[0] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn results_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "FightReport": {
                        "TotalScore": 123123,
                        "WinRate": 92,
                        "FightWin": 555,
                        "FightLose": 45,
                        "BeKilled": 2760854,
                        "Killed": 2759102,
                        "KillScore": 120378,
                        "FlagScore": 0,
                        "BuildingScore": 2745,
                        "GatherScore": 0,
                        "HealingScore": 0,
                        "HealingCnt": 0,
                        "FlagCnt": 0,
                        "RelocateCnt": 1,
                        "SpeedUpTime": 0,
                        "OccupyCnt": 2
                    }
                }
            }
        });

        let extractor = ResultsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();

        assert_eq!(fields["total_score"], json!(123123));
        assert_eq!(fields["win_rate"], json!(92));
        assert_eq!(fields["battles_win"], json!(555));
        assert_eq!(fields["battles_lose"], json!(45));
        assert_eq!(fields["severely_wounded"], json!(2760854));
        assert_eq!(fields["kills"], json!(2759102));
        assert_eq!(fields["kill_score"], json!(120378));
        assert_eq!(fields["flag_score"], json!(0));
        assert_eq!(fields["building_score"], json!(2745));
        assert_eq!(fields["gather_score"], json!(0));
        assert_eq!(fields["healing_score"], json!(0));
        assert_eq!(fields["units_healed"], json!(0));
        assert_eq!(fields["flag_count"], json!(0));
        assert_eq!(fields["teleports"], json!(1));
        assert_eq!(fields["speedups"], json!(0));
        assert_eq!(fields["structures"], json!(2));
    }

    #[test]
    fn results_extractor_accepts_fght_lose_alias() {
        let input = json!({
            "body": {
                "kvs": {
                    "FightReport": {
                        "TotalScore": 1,
                        "WinRate": 2,
                        "FightWin": 3,
                        "FghtLose": 4,
                        "BeKilled": 5,
                        "Killed": 6,
                        "KillScore": 7,
                        "FlagScore": 8,
                        "BuildingScore": 9,
                        "GatherScore": 10,
                        "HealingScore": 11,
                        "HealingCnt": 12,
                        "FlagCnt": 13,
                        "RelocateCnt": 14,
                        "SpeedUpTime": 15,
                        "OccupyCnt": 16
                    }
                }
            }
        });

        let extractor = ResultsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert_eq!(fields["battles_lose"], json!(4));
    }

    #[test]
    fn results_extractor_rejects_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "FightReport": {
                        "TotalScore": 123123
                    }
                }
            }
        });

        let extractor = ResultsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(
            err,
            ExtractError::MissingField { field: "WinRate" }
        ));
    }

    #[test]
    fn roundtrip_results_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ResultsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert_eq!(fields["total_score"], json!(123123));
        assert_eq!(fields["win_rate"], json!(92));
        assert_eq!(fields["battles_win"], json!(555));
        assert_eq!(fields["battles_lose"], json!(45));
        assert_eq!(fields["severely_wounded"], json!(2760854));
        assert_eq!(fields["kills"], json!(2759102));
        assert_eq!(fields["kill_score"], json!(120378));
        assert_eq!(fields["flag_score"], json!(0));
        assert_eq!(fields["building_score"], json!(2745));
        assert_eq!(fields["gather_score"], json!(0));
        assert_eq!(fields["healing_score"], json!(0));
        assert_eq!(fields["units_healed"], json!(0));
        assert_eq!(fields["flag_count"], json!(0));
        assert_eq!(fields["teleports"], json!(1));
        assert_eq!(fields["speedups"], json!(0));
        assert_eq!(fields["structures"], json!(2));
    }
}
