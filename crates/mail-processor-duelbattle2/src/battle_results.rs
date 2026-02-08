//! Battle results extractor for DuelBattle2 mail.

use mail_processor_sdk::{ExtractError, Extractor, Section};
use serde_json::{Map, Value, json};

use crate::player::{locate_player, require_bool_field, require_u64_field};

/// Extracts sender and opponent battle results from player payloads.
#[derive(Debug, Default)]
pub struct BattleResultsExtractor;

impl BattleResultsExtractor {
    /// Create a new battle results extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for BattleResultsExtractor {
    fn section(&self) -> &'static str {
        "battle_results"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let sender = locate_player(input, "AtkPlayer")?;
        let opponent = locate_player(input, "DefPlayer")?;

        let mut section = Section::new();
        section.insert("sender", extract_player_battle_results(sender)?);
        section.insert("opponent", extract_player_battle_results(opponent)?);
        Ok(section)
    }
}

fn extract_player_battle_results(player: &Map<String, Value>) -> Result<Value, ExtractError> {
    let win = require_bool_field(player, "IsWin")?;
    let kill_points = require_u64_field(player, "KillScore")?;
    let power = require_u64_field(player, "LosePower")?;
    let units = require_u64_field(player, "UnitTotal")?;
    let slightly_wounded = require_u64_field(player, "UnitHurt")?;
    let severely_wounded = require_u64_field(player, "UnitBadHurt")?;
    let dead = require_u64_field(player, "UnitDead")?;
    let heal = require_u64_field(player, "UnitReturn")?;

    Ok(json!({
        "win": win,
        "kill_points": kill_points,
        "power": power,
        "units": units,
        "slightly_wounded": slightly_wounded,
        "severely_wounded": severely_wounded,
        "dead": dead,
        "heal": heal,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_processor_sdk::Extractor;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn battle_results_extractor_reads_fields() {
        let input = json!({
            "body": {
                "detail": {
                    "AtkPlayer": {
                        "IsWin": true,
                        "KillScore": 10,
                        "LosePower": 20,
                        "UnitTotal": 30,
                        "UnitHurt": 40,
                        "UnitBadHurt": 50,
                        "UnitDead": 60,
                        "UnitReturn": 70
                    },
                    "DefPlayer": {
                        "IsWin": false,
                        "KillScore": 11,
                        "LosePower": 21,
                        "UnitTotal": 31,
                        "UnitHurt": 41,
                        "UnitBadHurt": 51,
                        "UnitDead": 61,
                        "UnitReturn": 71
                    }
                }
            }
        });
        let extractor = BattleResultsExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let fields = section.fields();
        assert_eq!(
            fields["sender"],
            json!({
                "win": true,
                "kill_points": 10,
                "power": 20,
                "units": 30,
                "slightly_wounded": 40,
                "severely_wounded": 50,
                "dead": 60,
                "heal": 70
            })
        );
        assert_eq!(
            fields["opponent"],
            json!({
                "win": false,
                "kill_points": 11,
                "power": 21,
                "units": 31,
                "slightly_wounded": 41,
                "severely_wounded": 51,
                "dead": 61,
                "heal": 71
            })
        );
    }

    #[test]
    fn roundtrip_battle_results_extracts_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = BattleResultsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let fields = section.fields();
        assert_eq!(
            fields["sender"],
            json!({
                "win": true,
                "kill_points": 999280,
                "power": 338240,
                "units": 430000,
                "slightly_wounded": 224608,
                "severely_wounded": 33824,
                "dead": 0,
                "heal": 0
            })
        );
        assert_eq!(
            fields["opponent"],
            json!({
                "win": false,
                "kill_points": 676480,
                "power": 499640,
                "units": 386173,
                "slightly_wounded": 336209,
                "severely_wounded": 49964,
                "dead": 0,
                "heal": 0
            })
        );
    }
}
