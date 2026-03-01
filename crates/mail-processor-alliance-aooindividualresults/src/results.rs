//! Results extractor for AllianceAOOIndividualResults mail.

use mail_processor_sdk::{ExtractError, Extractor, Section, require_object};
use serde_json::Value;

use crate::content::{optional_child_object, require_child_object, require_u64_field};

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
        let fight_report = optional_child_object(kvs, "FightReport")?;

        let mut section = Section::new();

        let total_score = fight_report
            .map(|value| require_u64_field(value, "TotalScore").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let win_rate = fight_report
            .map(|value| require_u64_field(value, "WinRate").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let battles_win = fight_report
            .map(|value| require_u64_field(value, "FightWin").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let battles_lose = fight_report
            .map(|value| require_u64_field(value, "FightLose").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let severely_wounded = fight_report
            .map(|value| require_u64_field(value, "BeKilled").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let kills = fight_report
            .map(|value| require_u64_field(value, "Killed").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let kill_score = fight_report
            .map(|value| require_u64_field(value, "KillScore").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let flag_score = fight_report
            .map(|value| require_u64_field(value, "FlagScore").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let building_score = fight_report
            .map(|value| require_u64_field(value, "BuildingScore").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let gather_score = fight_report
            .map(|value| require_u64_field(value, "GatherScore").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let healing_score = fight_report
            .map(|value| require_u64_field(value, "HealingScore").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let units_healed = fight_report
            .map(|value| require_u64_field(value, "HealingCnt").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let flag_count = fight_report
            .map(|value| require_u64_field(value, "FlagCnt").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let teleports = fight_report
            .map(|value| require_u64_field(value, "RelocateCnt").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let speedups = fight_report
            .map(|value| require_u64_field(value, "SpeedUpTime").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);
        let structures = fight_report
            .map(|value| require_u64_field(value, "OccupyCnt").map(Value::from))
            .transpose()?
            .unwrap_or(Value::Null);

        // Individual Points
        section.insert("total_score", total_score);
        // Win Percentage
        section.insert("win_rate", win_rate);
        section.insert("battles_win", battles_win);
        section.insert("battles_lose", battles_lose);
        section.insert("severely_wounded", severely_wounded);
        section.insert("kills", kills);
        section.insert("kill_score", kill_score);
        // Ark of Osiris Score
        section.insert("flag_score", flag_score);
        // Occupation Score
        section.insert("building_score", building_score);
        // Provisions Score
        section.insert("gather_score", gather_score);
        section.insert("healing_score", healing_score);
        section.insert("units_healed", units_healed);
        // Arks Captured
        section.insert("flag_count", flag_count);
        // teleports used
        section.insert("teleports", teleports);
        // minutes used
        section.insert("speedups", speedups);
        // structures reinforced
        section.insert("structures", structures);
        Ok(section)
    }
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
    fn results_extractor_allows_missing_fight_report() {
        let input = json!({
            "body": {
                "kvs": {
                    "Idx": 0
                }
            }
        });

        let extractor = ResultsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let fields = section.fields();
        assert!(fields["total_score"].is_null());
        assert!(fields["win_rate"].is_null());
        assert!(fields["battles_win"].is_null());
        assert!(fields["battles_lose"].is_null());
        assert!(fields["severely_wounded"].is_null());
        assert!(fields["kills"].is_null());
        assert!(fields["kill_score"].is_null());
        assert!(fields["flag_score"].is_null());
        assert!(fields["building_score"].is_null());
        assert!(fields["gather_score"].is_null());
        assert!(fields["healing_score"].is_null());
        assert!(fields["units_healed"].is_null());
        assert!(fields["flag_count"].is_null());
        assert!(fields["teleports"].is_null());
        assert!(fields["speedups"].is_null());
        assert!(fields["structures"].is_null());
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

    #[test]
    fn roundtrip_results_extracts_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Alliance/Persistent.Mail.6890312417293500508.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = ResultsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();

        assert!(fields["total_score"].is_null());
        assert!(fields["win_rate"].is_null());
        assert!(fields["battles_win"].is_null());
        assert!(fields["battles_lose"].is_null());
        assert!(fields["severely_wounded"].is_null());
        assert!(fields["kills"].is_null());
        assert!(fields["kill_score"].is_null());
        assert!(fields["flag_score"].is_null());
        assert!(fields["building_score"].is_null());
        assert!(fields["gather_score"].is_null());
        assert!(fields["healing_score"].is_null());
        assert!(fields["units_healed"].is_null());
        assert!(fields["flag_count"].is_null());
        assert!(fields["teleports"].is_null());
        assert!(fields["speedups"].is_null());
        assert!(fields["structures"].is_null());
    }
}
