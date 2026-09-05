//! Maps `body.kvs.FightReport.Stat.HerosStat` into commander pairing rows.
//!
//! A missing or null report, stat table, or pairing list produces an empty array;
//! `Stat = []` is also treated as absent. Present rows require both commander IDs
//! and their battle counters. Input order is preserved; pairings are not merged.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, require_array, require_object};
use serde_json::{Value, json};

use crate::content::{
    optional_child_object, optional_child_object_or_empty_array, require_child_object,
    require_u64_field,
};

/// Extracts hero pairing stats from `body.kvs.FightReport.Stat.HerosStat`.
#[derive(Debug, Default)]
pub struct PairingsExtractor;

impl PairingsExtractor {
    /// Creates a pairings extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for PairingsExtractor {
    fn section(&self) -> &'static str {
        "pairings"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let root = require_object(input)?;
        let body = require_child_object(root, "body")?;
        let kvs = require_child_object(body, "kvs")?;
        // Sparse reports can omit FightReport or encode its empty Stat table as [].
        // Both mean no recorded pairings, not a row with zero counters.
        let heroes_stat = match optional_child_object(kvs, "FightReport")? {
            Some(fight_report) => match optional_child_object_or_empty_array(fight_report, "Stat")?
            {
                Some(stat) => match stat.get("HerosStat") {
                    None | Some(Value::Null) => &[],
                    Some(value) => require_array(value, "HerosStat")?,
                },
                None => &[],
            },
            None => &[],
        };

        let mut pairings = Vec::with_capacity(heroes_stat.len());
        for pairing in heroes_stat {
            let pairing = pairing
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field: "HerosStat", expected: "object" })?;
            let primary_id = require_u64_field(pairing, "MainHeroId")?;
            let secondary_id = require_u64_field(pairing, "AssistHeroId")?;
            let kill_count = require_u64_field(pairing, "KillCnt")?;
            let battles_win = require_u64_field(pairing, "AllBattleWinCnt")?;
            let all_battle_stat = require_child_object(pairing, "AllBattleStat")?;
            let battles = require_u64_field(all_battle_stat, "BattleCnt")?;
            let severely_wounded = require_u64_field(all_battle_stat, "BeKilledScore")?;
            let kill_points = require_u64_field(all_battle_stat, "KillScore")?;

            pairings.push(json!({
                "primary_commander": { "id": primary_id },
                "secondary_commander": { "id": secondary_id },
                "kill_count": kill_count,
                "battles_win": battles_win,
                "battles": battles,
                "severely_wounded": severely_wounded,
                "kill_points": kill_points,
            }));
        }

        Ok(Section::from_array(pairings))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rokbattles_mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn pairings_extractor_reads_fields() {
        let input = json!({
            "body": {
                "kvs": {
                    "FightReport": {
                        "Stat": {
                            "HerosStat": [
                                {
                                    "MainHeroId": 540,
                                    "AssistHeroId": 459,
                                    "KillCnt": 376786,
                                    "AllBattleWinCnt": 108,
                                    "AllBattleStat": {
                                        "BattleCnt": 252,
                                        "BeKilledScore": 11681020,
                                        "KillScore": 7344000
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });

        let extractor = PairingsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let pairings = section.array().expect("pairings");

        assert_eq!(pairings.len(), 1);
        assert_eq!(pairings[0]["primary_commander"]["id"], json!(540));
        assert_eq!(pairings[0]["secondary_commander"]["id"], json!(459));
        assert_eq!(pairings[0]["kill_count"], json!(376786));
        assert_eq!(pairings[0]["battles_win"], json!(108));
        assert_eq!(pairings[0]["battles"], json!(252));
        assert_eq!(pairings[0]["severely_wounded"], json!(11681020));
        assert_eq!(pairings[0]["kill_points"], json!(7344000));
    }

    #[test]
    fn pairings_extractor_allows_missing_field() {
        let input = json!({
            "body": {
                "kvs": {
                    "FightReport": {
                        "Stat": {}
                    }
                }
            }
        });

        let extractor = PairingsExtractor::new();
        let section = extractor.extract(&input).unwrap();
        let pairings = section.array().expect("pairings");
        assert!(pairings.is_empty());
    }

    #[test]
    fn roundtrip_pairings_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = PairingsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let pairings = section.array().expect("pairings");

        assert_eq!(pairings.len(), 6);
        assert_eq!(pairings[0]["primary_commander"]["id"], json!(540));
        assert_eq!(pairings[0]["secondary_commander"]["id"], json!(459));
        assert_eq!(pairings[0]["kill_count"], json!(376786));
        assert_eq!(pairings[0]["battles_win"], json!(108));
        assert_eq!(pairings[0]["battles"], json!(252));
        assert_eq!(pairings[0]["severely_wounded"], json!(11681020));
        assert_eq!(pairings[0]["kill_points"], json!(7344000));
        assert_eq!(pairings[5]["primary_commander"]["id"], json!(179));
        assert_eq!(pairings[5]["secondary_commander"]["id"], json!(187));
        assert_eq!(pairings[5]["kill_count"], json!(105255));
        assert_eq!(pairings[5]["battles_win"], json!(30));
        assert_eq!(pairings[5]["battles"], json!(77));
        assert_eq!(pairings[5]["severely_wounded"], json!(3852540));
        assert_eq!(pairings[5]["kill_points"], json!(2063230));
    }

    #[test]
    fn roundtrip_pairings_extracts_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.6890312417293500508.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = PairingsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let pairings = section.array().expect("pairings");
        assert!(pairings.is_empty());
    }

    #[test]
    fn roundtrip_pairings_extracts_empty_stat_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.91536773174395176822.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = PairingsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let pairings = section.array().expect("pairings");
        assert!(pairings.is_empty());
    }
}
