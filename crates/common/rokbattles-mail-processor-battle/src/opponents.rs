//! Builds one opponent row per entry in `body.content.Attacks`.
//!
//! Each attack is extracted on its own scoped thread within the SDK section worker.
//! Rows are sorted by the leading numeric portion of the attack key, then by the
//! full key. The original key is retained in output so suffixed attacks stay distinct.
//!
//! Rows combine character details, attack ticks and position, participants, NPC
//! loot, per-side results, and effects. Missing result blocks become all-null
//! objects; missing NPC loot remains null rather than an empty recorded drop list.

use rokbattles_mail_sdk::{
    ExtractError, Extractor, Section, optional_i64_field, optional_u64_field, require_array,
    require_i64_field, require_number_field,
};
use serde_json::{Map, Value, json};

use crate::{
    battle_effects::extract_battle_effects,
    content::{require_child_object, require_content, require_u64_field},
    participants::extract_participants,
    player::extract_player_fields,
};

/// Extracts opponent details out of each attack entry.
#[derive(Debug, Default)]
pub struct OpponentsExtractor;

impl OpponentsExtractor {
    /// Creates an opponents extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for OpponentsExtractor {
    fn section(&self) -> &'static str {
        "opponents"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        let content = require_content(input)?;
        let attacks = require_attacks(content)?;

        let mut results = Vec::with_capacity(attacks.len());
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(attacks.len());
            for (attack_key, attack) in attacks {
                let attack_key = attack_key.to_string();
                let handle = scope.spawn(move || extract_attack_entry(attack_key, attack));
                handles.push(handle);
            }

            for handle in handles {
                let result =
                    handle.join().map_err(|_panic_payload| ExtractError::InvalidFieldType {
                        field: "Attacks",
                        expected: "non-panicking extraction",
                    })?;
                results.push(result?);
            }
            Ok::<(), ExtractError>(())
        })?;

        // Worker completion order and lexicographic JSON keys do not define attack order.
        // Keep the full key as a tie-breaker when attacks share a numeric prefix.
        results.sort_by(|(attack_id_a, attack_key_a, _), (attack_id_b, attack_key_b, _)| {
            attack_id_a.cmp(attack_id_b).then_with(|| attack_key_a.cmp(attack_key_b))
        });
        let entries = results.into_iter().map(|(_, _, value)| value).collect();

        Ok(Section::from_array(entries))
    }
}

/// Returns the attacks map from the content object.
fn require_attacks(content: &Map<String, Value>) -> Result<&Map<String, Value>, ExtractError> {
    let value = content.get("Attacks").ok_or(ExtractError::MissingField { field: "Attacks" })?;
    value.as_object().ok_or(ExtractError::InvalidFieldType { field: "Attacks", expected: "object" })
}

/// Parses the leading ASCII digits of an attack key for numeric ordering.
///
/// Suffixes are allowed and retained separately; a key without leading digits
/// or with an overflowing prefix fails extraction.
fn parse_attack_id(attack_id: &str) -> Result<u64, ExtractError> {
    let prefix = attack_id.split(|ch: char| !ch.is_ascii_digit()).next().unwrap_or_default();
    prefix.parse::<u64>().map_err(|_parse_error| ExtractError::InvalidFieldType {
        field: "Attacks",
        expected: "numeric object key",
    })
}

/// Builds one opponent entry from an attack payload.
fn extract_attack_entry(
    attack_key: String,
    attack: &Value,
) -> Result<(u64, String, Value), ExtractError> {
    let attack = attack
        .as_object()
        .ok_or(ExtractError::InvalidFieldType { field: "Attacks", expected: "object" })?;
    let opponent = require_child_object(attack, "CIdt")?;
    let mut fields = extract_player_fields(opponent)?;
    let attack_id = parse_attack_id(&attack_key)?;
    let position = require_child_object(attack, "Pos")?;
    let attack_x = require_number_field(position, "X")?;
    let attack_y = require_number_field(position, "Y")?;
    let (start_tick, end_tick) = extract_attack_tick_bounds(attack)?;
    fields.insert(
        "attack".to_string(),
        json!({ "id": attack_key.clone(), "x": attack_x, "y": attack_y }),
    );
    fields.insert("start_tick".to_string(), Value::from(start_tick));
    fields.insert("end_tick".to_string(), Value::from(end_tick));
    let participants = extract_participants(attack, "OTs")?;
    fields.insert("participants".to_string(), participants);
    let npc = extract_npc(attack, opponent)?;
    fields.insert("npc".to_string(), npc);
    let battle_results = extract_battle_results(attack)?;
    fields.insert("battle_results".to_string(), battle_results);
    let battle_effects = extract_battle_effects(attack)?;
    fields.insert("battle_effects".to_string(), battle_effects);
    Ok((attack_id, attack_key, Value::Object(fields)))
}

/// Copies attack-local `Bts` and `Ets` into start and end ticks.
///
/// These fields come from the attack object, not the report-level timestamp
/// fields with the same names. No unit conversion or range ordering is applied.
fn extract_attack_tick_bounds(attack: &Map<String, Value>) -> Result<(u64, u64), ExtractError> {
    let start_tick = require_u64_field(attack, "Bts")?;
    let end_tick = require_u64_field(attack, "Ets")?;
    Ok((start_tick, end_tick))
}

/// Extracts NPC-related data from the attack entry and opponent payload.
fn extract_npc(
    attack: &Map<String, Value>,
    opponent: &Map<String, Value>,
) -> Result<Value, ExtractError> {
    let npc_type = optional_u64_field(opponent, "NpcType")?;
    let npc_b_type = optional_u64_field(opponent, "NpcBType")?;
    let experience = optional_u64_field(attack, "NpcAtkExp")?;
    let loot = match extract_npc_loot(attack)? {
        Some(entries) => Value::Array(entries),
        None => Value::Null,
    };

    Ok(json!({
        "type": npc_type,
        "b_type": npc_b_type,
        "experience": experience,
        "loot": loot,
    }))
}

/// Maps `Damage` to sender results and `Kill` to opponent results.
///
/// These are reported per-side counters, not values recomputed from summaries.
fn extract_battle_results(attack: &Map<String, Value>) -> Result<Value, ExtractError> {
    let sender = extract_battle_result_optional(attack.get("Damage"), "Damage")?;
    let opponent = extract_battle_result_optional(attack.get("Kill"), "Kill")?;
    Ok(json!({ "sender": sender, "opponent": opponent }))
}

/// Normalizes one battle result entry into the output schema.
fn extract_battle_result(overview: &Map<String, Value>) -> Result<Value, ExtractError> {
    let reinforcements_join = require_u64_field(overview, "AddCnt")?;
    let reinforcements_leave = require_u64_field(overview, "RetreatCnt")?;
    // Preserve the legacy zero default for a missing or null KillScore. A wholly
    // absent result block takes the separate all-null path below.
    let kill_points = optional_u64_field(overview, "KillScore")?.unwrap_or(0);
    let acclaim = optional_u64_field(overview, "Contribute")?;
    let severely_wounded = require_u64_field(overview, "BadHurt")?;
    let slightly_wounded = require_i64_field(overview, "Hurt")?;
    let remaining = require_u64_field(overview, "Cnt")?;
    let dead = require_u64_field(overview, "Death")?;
    let heal = require_u64_field(overview, "Healing")?;
    let troop_units = require_u64_field(overview, "Max")?;
    let troop_units_max = require_u64_field(overview, "InitMax")?;
    let watchtower_max = require_u64_field(overview, "GtMax")?;
    let watchtower = require_u64_field(overview, "Gt")?;
    let power = require_i64_field(overview, "Power")?;
    // Missing power components default to zero, while signed values remain signed.
    let attack_power = optional_i64_field(overview, "AtkPower")?.unwrap_or(0);
    let skill_power = optional_i64_field(overview, "SkillPower")?.unwrap_or(0);
    // Missing newer counters stay null so absence is distinguishable from zero.
    let merits = optional_u64_field(overview, "WarExploits")?;
    let death_reduction = optional_u64_field(overview, "DeadReduceCnt")?;
    let severe_wound_reduction = optional_u64_field(overview, "BadReduceCnt")?;

    Ok(json!({
        "reinforcements_join": reinforcements_join,
        "reinforcements_leave": reinforcements_leave,
        "kill_points": kill_points,
        "acclaim": acclaim.map(Value::from).unwrap_or(Value::Null),
        "severely_wounded": severely_wounded,
        "slightly_wounded": slightly_wounded,
        "remaining": remaining,
        "dead": dead,
        "heal": heal,
        "troop_units": troop_units,
        "troop_units_max": troop_units_max,
        "watchtower_max": watchtower_max,
        "watchtower": watchtower,
        "power": power,
        "attack_power": attack_power,
        "skill_power": skill_power,
        "merits": merits.map(Value::from).unwrap_or(Value::Null),
        "death_reduction": death_reduction.map(Value::from).unwrap_or(Value::Null),
        "severe_wound_reduction": severe_wound_reduction.map(Value::from).unwrap_or(Value::Null),
    }))
}

/// Reads a battle result object when present, or returns an all-null entry.
fn extract_battle_result_optional(
    value: Option<&Value>,
    field: &'static str,
) -> Result<Value, ExtractError> {
    match value {
        None | Some(Value::Null) => Ok(null_battle_result()),
        Some(value) => {
            let overview = value
                .as_object()
                .ok_or(ExtractError::InvalidFieldType { field, expected: "object" })?;
            extract_battle_result(overview)
        }
    }
}

/// Builds an all-null battle result entry when the payload is missing.
fn null_battle_result() -> Value {
    json!({
        "reinforcements_join": Value::Null,
        "reinforcements_leave": Value::Null,
        "kill_points": Value::Null,
        "acclaim": Value::Null,
        "severely_wounded": Value::Null,
        "slightly_wounded": Value::Null,
        "remaining": Value::Null,
        "dead": Value::Null,
        "heal": Value::Null,
        "troop_units": Value::Null,
        "troop_units_max": Value::Null,
        "watchtower_max": Value::Null,
        "watchtower": Value::Null,
        "power": Value::Null,
        "attack_power": Value::Null,
        "skill_power": Value::Null,
        "merits": Value::Null,
        "death_reduction": Value::Null,
        "severe_wound_reduction": Value::Null,
    })
}

/// Reads NPC loot drops when they are present on the attack payload.
fn extract_npc_loot(attack: &Map<String, Value>) -> Result<Option<Vec<Value>>, ExtractError> {
    let value = match attack.get("NpcKillLoot") {
        None | Some(Value::Null) => return Ok(None),
        Some(value) => value,
    };

    let values = require_array(value, "NpcKillLoot")?;
    let mut loot = Vec::with_capacity(values.len());
    for entry in values {
        let entry = entry
            .as_object()
            .ok_or(ExtractError::InvalidFieldType { field: "NpcKillLoot", expected: "object" })?;
        let loot_type = require_u64_field(entry, "Type")?;
        let sub_type = require_u64_field(entry, "SubType")?;
        let value = require_u64_field(entry, "Value")?;
        loot.push(json!({
            "type": loot_type,
            "sub_type": sub_type,
            "value": value,
        }));
    }

    Ok(Some(loot))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rokbattles_mail_sdk::Extractor;
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn opponents_extractor_reads_attacks() {
        let input = json!({
            "body": {
                "content": {
                    "Attacks": {
                        "10": {
                            "Bts": 110,
                            "Ets": 120,
                            "Pos": { "X": 12.5, "Y": 34.75 },
                            "OTs": {
                                "5": {
                                    "PId": 500,
                                    "PName": "OT-One",
                                    "Abbr": "OT",
                                    "HId": 1,
                                    "HLv": 2,
                                    "HId2": 3,
                                    "HLv2": 4
                                }
                            },
                            "CIdt": {
                                "PId": 1,
                                "PName": "EnemyOne",
                                "COSId": 101,
                                "AId": 1,
                                "AName": "AllianceOne",
                                "Abbr": "ONE",
                                "AbT": 3,
                                "CastlePos": { "X": 50, "Y": 60 },
                                "CastleLevel": 20,
                                "GtLevel": 7,
                                "CTK": "tracking-4",
                                "ShId": 22,
                                "IsRally": false,
                                "CASS": {
                                    "ENABLE": true,
                                    "SKILLS": [
                                        { "HeroId": 62, "SkillId": 1132, "SkillLevel": 5 }
                                    ]
                                },
                                "AppUid": "103134073",
                                "Avatar": "https://example.com/one.png"
                            }
                        },
                        "20": {
                            "Bts": 210,
                            "Ets": 220,
                            "Pos": { "X": 98, "Y": 76 },
                            "OTs": {
                                "-2": {
                                    "PId": 600,
                                    "PName": "OT-Two",
                                    "Abbr": "OT2",
                                    "HId": 5,
                                    "HLv": 6,
                                    "HId2": 7,
                                    "HLv2": 8
                                }
                            },
                            "CIdt": {
                                "PId": 2,
                                "PName": "EnemyTwo",
                                "COSId": 202,
                                "AId": 2,
                                "AName": "AllianceTwo",
                                "Abbr": "TWO",
                                "AbT": 11,
                                "CastlePos": { "X": 70.5, "Y": 80.25 },
                                "CastleLevel": 18,
                                "CTK": "",
                                "SideId": 9,
                                "ShId": 51,
                                "IsRally": true,
                                "AppUid": "8518744-399975",
                                "Avatar": "{\"avatar\":\"https://example.com/two.png\",\"avatarFrame\":\"https://example.com/frame.png\"}"
                            }
                        }
                    }
                }
            }
        });
        let extractor = OpponentsExtractor::new();
        let section = extractor.extract(&input).unwrap();

        let opponents = section.array().expect("opponents array");
        assert_eq!(opponents.len(), 2);
        assert_eq!(opponents[0]["attack"]["id"], json!("10"));
        assert_eq!(opponents[0]["start_tick"], json!(110));
        assert_eq!(opponents[0]["end_tick"], json!(120));
        assert_eq!(opponents[0]["attack"]["x"], json!(12.5));
        assert_eq!(opponents[0]["attack"]["y"], json!(34.75));
        assert_eq!(opponents[0]["player_id"], json!(1));
        assert_eq!(opponents[0]["player_name"], json!("EnemyOne"));
        assert_eq!(opponents[0]["kingdom_id"], json!(101));
        assert_eq!(
            opponents[0]["alliance"],
            json!({ "id": 1, "name": "AllianceOne", "abbreviation": "ONE" })
        );
        assert_eq!(opponents[0]["alliance_building_id"], json!(3));
        assert_eq!(
            opponents[0]["castle"],
            json!({
                "x": 50,
                "y": 60,
                "level": 20,
                "watchtower": 7
            })
        );
        assert_eq!(opponents[0]["tracking_key"], json!("tracking-4"));
        assert_eq!(opponents[0]["camp_id"], json!(null));
        assert_eq!(opponents[0]["rally"], json!(false));
        assert_eq!(opponents[0]["structure_id"], json!(22));
        assert!(opponents[0]["commanders"]["primary"]["id"].is_null());
        assert!(opponents[0]["commanders"]["secondary"]["id"].is_null());
        assert_eq!(
            opponents[0]["support_skills"],
            json!({
                "enable": true,
                "skills": [{ "hero_id": 62, "skill_id": 1132, "skill_level": 5 }]
            })
        );
        assert_eq!(opponents[0]["app_id"], json!(2104267));
        assert_eq!(opponents[0]["app_uid"], json!(103134073));
        let participants = opponents[0]["participants"].as_array().expect("participants array");
        assert_eq!(participants.len(), 1);
        assert_eq!(
            participants[0],
            json!({
                "participant_id": 5,
                "player_id": 500,
                "player_name": "OT-One",
                "alliance": { "abbreviation": "OT" },
                "commanders": {
                    "primary": { "id": 1, "level": 2 },
                    "secondary": { "id": 3, "level": 4 },
                }
            })
        );
        assert_eq!(opponents[1]["attack"]["id"], json!("20"));
        assert_eq!(opponents[1]["start_tick"], json!(210));
        assert_eq!(opponents[1]["end_tick"], json!(220));
        assert_eq!(opponents[1]["attack"]["x"], json!(98));
        assert_eq!(opponents[1]["attack"]["y"], json!(76));
        assert_eq!(opponents[1]["player_id"], json!(2));
        assert_eq!(
            opponents[1]["alliance"],
            json!({ "id": 2, "name": "AllianceTwo", "abbreviation": "TWO" })
        );
        assert_eq!(opponents[1]["alliance_building_id"], json!(11));
        assert_eq!(
            opponents[1]["castle"],
            json!({
                "x": 70.5,
                "y": 80.25,
                "level": 18,
                "watchtower": null
            })
        );
        assert_eq!(opponents[1]["tracking_key"], json!(""));
        assert_eq!(opponents[1]["camp_id"], json!(9));
        assert_eq!(opponents[1]["rally"], json!(true));
        assert_eq!(opponents[1]["structure_id"], json!(51));
        assert!(opponents[1]["commanders"]["primary"]["id"].is_null());
        assert!(opponents[1]["commanders"]["secondary"]["id"].is_null());
        assert_eq!(opponents[1]["support_skills"], json!({ "enable": false, "skills": [] }));
        assert_eq!(opponents[1]["app_id"], json!(8518744));
        assert_eq!(opponents[1]["app_uid"], json!(399975));
        assert_eq!(opponents[1]["frame_url"], json!("https://example.com/frame.png"));
        let participants = opponents[1]["participants"].as_array().expect("participants array");
        assert_eq!(participants.len(), 1);
        assert_eq!(
            participants[0],
            json!({
                "participant_id": -2,
                "player_id": 600,
                "player_name": "OT-Two",
                "alliance": { "abbreviation": "OT2" },
                "commanders": {
                    "primary": { "id": 5, "level": 6 },
                    "secondary": { "id": 7, "level": 8 },
                }
            })
        );
    }

    #[test]
    fn opponents_extractor_rejects_missing_attacks() {
        let input = json!({ "body": { "content": {} } });
        let extractor = OpponentsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { .. }));
    }

    #[test]
    fn opponents_extractor_rejects_missing_attack_tick_bounds() {
        let input = json!({
            "body": {
                "content": {
                    "Attacks": {
                        "10": {
                            "Ets": 120,
                            "Pos": { "X": 12.5, "Y": 34.75 },
                            "OTs": {},
                            "CIdt": {
                                "PId": 1,
                                "PName": "EnemyOne",
                                "COSId": 101,
                                "AId": 1,
                                "AName": "AllianceOne",
                                "Abbr": "ONE",
                                "AbT": 3,
                                "CastlePos": { "X": 50, "Y": 60 },
                                "CastleLevel": 20,
                                "CTK": "",
                                "Avatar": "https://example.com/one.png"
                            }
                        }
                    }
                }
            }
        });
        let extractor = OpponentsExtractor::new();
        let err = extractor.extract(&input).unwrap_err();
        assert!(matches!(err, ExtractError::MissingField { field: "Bts" }));
    }

    #[test]
    fn parse_attack_id_allows_suffixes() {
        assert_eq!(parse_attack_id("79272307").unwrap(), 79272307);
        assert_eq!(parse_attack_id("79272307_1").unwrap(), 79272307);
        assert_eq!(parse_attack_id("79272307_extra").unwrap(), 79272307);
        assert_eq!(parse_attack_id("79272307敵").unwrap(), 79272307);
    }

    #[test]
    fn parse_attack_id_rejects_missing_or_overflowing_prefixes() {
        for key in ["", "敵79272307", "-1", "18446744073709551616"] {
            assert_eq!(
                parse_attack_id(key),
                Err(ExtractError::InvalidFieldType {
                    field: "Attacks",
                    expected: "numeric object key",
                }),
            );
        }
    }

    #[test]
    fn extract_battle_results_reads_values() {
        let attack = json!({
            "Damage": {
                "AddCnt": 1,
                "RetreatCnt": 2,
                "KillScore": 3,
                "Contribute": 4,
                "BadHurt": 4,
                "Hurt": 5,
                "Cnt": 6,
                "Death": 7,
                "Healing": 8,
                "Max": 9,
                "InitMax": 10,
                "GtMax": 11,
                "Gt": 12,
                "Power": -13,
                "AtkPower": -14,
                "SkillPower": 15,
                "WarExploits": 16,
                "DeadReduceCnt": 17,
                "BadReduceCnt": 18
            },
            "Kill": {
                "AddCnt": 13,
                "RetreatCnt": 14,
                "KillScore": 15,
                "Contribute": 16,
                "BadHurt": 17,
                "Hurt": 18,
                "Cnt": 19,
                "Death": 20,
                "Healing": 21,
                "Max": 22,
                "InitMax": 23,
                "GtMax": 24,
                "Gt": 25,
                "Power": -26,
                "AtkPower": -27,
                "SkillPower": 28,
                "WarExploits": 29,
                "DeadReduceCnt": 30,
                "BadReduceCnt": 31
            }
        });
        let results = extract_battle_results(attack.as_object().unwrap()).expect("results");
        assert_eq!(
            results,
            json!({
                "sender": {
                    "reinforcements_join": 1,
                    "reinforcements_leave": 2,
                    "kill_points": 3,
                    "acclaim": 4,
                    "severely_wounded": 4,
                    "slightly_wounded": 5,
                    "remaining": 6,
                    "dead": 7,
                    "heal": 8,
                    "troop_units": 9,
                    "troop_units_max": 10,
                    "watchtower_max": 11,
                    "watchtower": 12,
                    "power": -13,
                    "attack_power": -14,
                    "skill_power": 15,
                    "merits": 16,
                    "death_reduction": 17,
                    "severe_wound_reduction": 18
                },
                "opponent": {
                    "reinforcements_join": 13,
                    "reinforcements_leave": 14,
                    "kill_points": 15,
                    "acclaim": 16,
                    "severely_wounded": 17,
                    "slightly_wounded": 18,
                    "remaining": 19,
                    "dead": 20,
                    "heal": 21,
                    "troop_units": 22,
                    "troop_units_max": 23,
                    "watchtower_max": 24,
                    "watchtower": 25,
                    "power": -26,
                    "attack_power": -27,
                    "skill_power": 28,
                    "merits": 29,
                    "death_reduction": 30,
                    "severe_wound_reduction": 31
                }
            })
        );
    }

    #[test]
    fn extract_battle_results_handles_missing_payloads() {
        let attack = json!({});
        let results = extract_battle_results(attack.as_object().unwrap()).expect("results");
        assert_eq!(results["sender"], null_battle_result());
        assert_eq!(results["opponent"], null_battle_result());
    }

    #[test]
    fn extract_battle_results_defaults_missing_kill_score() {
        let attack = json!({
            "Damage": {
                "AddCnt": 1,
                "RetreatCnt": 2,
                "Contribute": 4,
                "BadHurt": 4,
                "Hurt": 5,
                "Cnt": 6,
                "Death": 7,
                "Healing": 8,
                "Max": 9,
                "InitMax": 10,
                "GtMax": 11,
                "Gt": 12,
                "Power": -13,
                "AtkPower": -14,
                "SkillPower": 15
            }
        });
        let results = extract_battle_results(attack.as_object().unwrap()).expect("results");
        assert_eq!(results["sender"]["kill_points"], json!(0));
        assert!(results["sender"]["merits"].is_null());
        assert!(results["sender"]["death_reduction"].is_null());
        assert!(results["sender"]["severe_wound_reduction"].is_null());
    }

    #[test]
    fn extract_battle_results_defaults_missing_attack_power() {
        let attack = json!({
            "Damage": {
                "AddCnt": 1,
                "RetreatCnt": 2,
                "BadHurt": 3,
                "Hurt": 4,
                "Cnt": 5,
                "Death": 6,
                "Healing": 7,
                "Max": 8,
                "InitMax": 9,
                "GtMax": 10,
                "Gt": 11,
                "Power": -12
            },
            "Kill": {
                "AddCnt": 13,
                "RetreatCnt": 14,
                "BadHurt": 15,
                "Hurt": 16,
                "Cnt": 17,
                "Death": 18,
                "Healing": 19,
                "Max": 20,
                "InitMax": 21,
                "GtMax": 22,
                "Gt": 23,
                "Power": -24
            }
        });
        let results = extract_battle_results(attack.as_object().unwrap()).expect("results");
        assert_eq!(results["sender"]["attack_power"], json!(0));
        assert_eq!(results["sender"]["skill_power"], json!(0));
        assert_eq!(results["opponent"]["attack_power"], json!(0));
        assert_eq!(results["opponent"]["skill_power"], json!(0));
    }

    #[test]
    fn extract_battle_results_preserves_negative_wounded_counts() {
        let attack = json!({
            "Damage": {
                "AddCnt": 1,
                "RetreatCnt": 2,
                "BadHurt": 3,
                "Hurt": -4,
                "Cnt": 5,
                "Death": 6,
                "Healing": 7,
                "Max": 8,
                "InitMax": 9,
                "GtMax": 10,
                "Gt": 11,
                "Power": -12
            },
            "Kill": {
                "AddCnt": 13,
                "RetreatCnt": 14,
                "BadHurt": 15,
                "Hurt": -16,
                "Cnt": 17,
                "Death": 18,
                "Healing": 19,
                "Max": 20,
                "InitMax": 21,
                "GtMax": 22,
                "Gt": 23,
                "Power": -24
            }
        });
        let results = extract_battle_results(attack.as_object().unwrap()).expect("results");
        assert_eq!(results["sender"]["severely_wounded"], json!(3));
        assert_eq!(results["sender"]["slightly_wounded"], json!(-4));
        assert_eq!(results["opponent"]["severely_wounded"], json!(15));
        assert_eq!(results["opponent"]["slightly_wounded"], json!(-16));
    }

    #[test]
    fn opponents_extractor_attaches_schema_819_battle_effects() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.13168813178320161112.json");
        let input = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&input).expect("parse sample");

        let section = OpponentsExtractor::new().extract(&value).expect("extract opponents");
        let opponents = section.array().expect("opponents array");
        let entry = opponents
            .iter()
            .find(|opponent| opponent["attack"]["id"] == json!("180150801"))
            .expect("target attack entry");

        assert_eq!(
            entry["battle_effects"],
            json!({
                "sender": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [24, 22, 28, 30, 31, 23] }
                    ],
                    "statistics": []
                },
                "opponent": {
                    "modifier_sources": [
                        { "source": "policy_v2", "ids": [11, 19, 7, 12, 2, 14] }
                    ],
                    "statistics": [
                        {
                            "source": "policy_v2",
                            "id": 2,
                            "stats": [
                                { "key": "ExtraBadHurt", "value": 719 }
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn roundtrip_opponents_extract_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OpponentsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let opponents = section.array().expect("opponents array");
        assert_eq!(opponents.len(), 1);
        assert_eq!(opponents[0]["attack"]["id"], json!("603103"));
        assert_eq!(opponents[0]["start_tick"], json!(312472));
        assert_eq!(opponents[0]["end_tick"], json!(312472));
        assert_eq!(opponents[0]["attack"]["x"], json!(2200.1484));
        assert_eq!(opponents[0]["attack"]["y"], json!(6296.3677));
        assert_eq!(opponents[0]["player_id"], json!(130014943));
        assert_eq!(opponents[0]["player_name"], json!("xAvarice"));
        assert_eq!(opponents[0]["alliance"], json!({ "id": 0, "name": "", "abbreviation": "" }));
        assert!(opponents[0]["alliance_building_id"].is_null());
        assert_eq!(opponents[0]["tracking_key"], json!(""));
        assert_eq!(
            opponents[0]["castle"],
            json!({
                "x": 2206.4922,
                "y": 6292.705,
                "level": 24,
                "watchtower": 17
            })
        );
        assert_eq!(opponents[0]["camp_id"], json!(null));
        assert!(opponents[0]["rally"].is_null());
        assert!(opponents[0]["structure_id"].is_null());
        assert_eq!(opponents[0]["commanders"]["primary"]["id"], json!(9));
        assert_eq!(opponents[0]["commanders"]["primary"]["awakened"], json!(false));
        assert_eq!(opponents[0]["commanders"]["primary"]["star_level"], json!(5));
        assert_eq!(opponents[0]["commanders"]["primary"]["relics"], json!(null));
        assert_eq!(opponents[0]["commanders"]["primary"]["armaments"], json!(null));
        assert_eq!(opponents[0]["commanders"]["secondary"]["id"], json!(7));
        assert_eq!(opponents[0]["commanders"]["secondary"]["awakened"], json!(false));
        assert_eq!(opponents[0]["commanders"]["secondary"]["star_level"], json!(5));
        assert_eq!(opponents[0]["commanders"]["secondary"]["relics"], json!(null));
        assert_eq!(opponents[0]["commanders"]["secondary"]["armaments"], json!(null));
        assert_eq!(opponents[0]["npc"]["type"], json!(null));
        assert_eq!(opponents[0]["npc"]["b_type"], json!(null));
        assert_eq!(opponents[0]["npc"]["experience"], json!(0));
        assert_eq!(opponents[0]["npc"]["loot"], json!(null));
        let participants = opponents[0]["participants"].as_array().expect("participants array");
        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0]["participant_id"], json!(-2));
        assert_eq!(participants[0]["player_id"], json!(130014943));
        assert_eq!(participants[1]["participant_id"], json!(0));
    }

    #[test]
    fn opponents_extractor_preserves_is_auto_one_from_player() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let mut value: Value = serde_json::from_str(&json).expect("parse sample");
        value["body"]["content"]["Attacks"]["603103"]["CIdt"]["IsAuto"] = json!(1);

        let section = OpponentsExtractor::new().extract(&value).expect("extract sample");
        let opponents = section.array().expect("opponents array");

        assert_eq!(opponents[0]["is_auto"], json!(1));
    }

    #[test]
    fn opponents_extractor_reads_session_from_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.16210617176935008431.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OpponentsExtractor::new();

        let section = extractor.extract(&value).expect("extract sample");
        let opponents = section.array().expect("opponents array");

        assert_eq!(opponents[0]["session"], json!("cfg_id=4&id=3538&mode=ab&role_id=113&submode="));
    }

    #[test]
    fn roundtrip_opponents_allows_negative_player_id() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Battle/Persistent.Mail.1409019176893142331.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        let extractor = OpponentsExtractor::new();
        let section = extractor.extract(&value).expect("extract sample");
        let opponents = section.array().expect("opponents array");
        assert_eq!(opponents.len(), 1);
        assert_eq!(opponents[0]["attack"]["id"], json!("10852801"));
        assert_eq!(opponents[0]["start_tick"], json!(35058));
        assert_eq!(opponents[0]["end_tick"], json!(35068));
        assert_eq!(opponents[0]["attack"]["x"], json!(3804.8684));
        assert_eq!(opponents[0]["attack"]["y"], json!(3974.8313));
        assert_eq!(opponents[0]["player_id"], json!(-2));
        assert_eq!(opponents[0]["alliance"], json!({ "id": 0, "name": "", "abbreviation": "" }));
        assert!(opponents[0]["alliance_building_id"].is_null());
        assert_eq!(opponents[0]["tracking_key"], json!("-2_1768917586_86"));
        assert_eq!(
            opponents[0]["castle"],
            json!({
                "x": 3803.5735,
                "y": 3981.8271,
                "level": 0,
                "watchtower": null
            })
        );
        assert_eq!(opponents[0]["camp_id"], json!(null));
        assert!(opponents[0]["rally"].is_null());
        assert!(opponents[0]["structure_id"].is_null());
        assert_eq!(opponents[0]["commanders"]["primary"]["id"], json!(86));
        assert_eq!(opponents[0]["commanders"]["primary"]["awakened"], json!(false));
        assert_eq!(opponents[0]["commanders"]["primary"]["star_level"], json!(5));
        assert_eq!(opponents[0]["commanders"]["primary"]["relics"], json!(null));
        assert_eq!(opponents[0]["commanders"]["primary"]["armaments"], json!(null));
        assert_eq!(opponents[0]["commanders"]["secondary"]["id"], json!(0));
        assert_eq!(opponents[0]["commanders"]["secondary"]["awakened"], json!(false));
        assert_eq!(opponents[0]["commanders"]["secondary"]["star_level"], json!(0));
        assert_eq!(opponents[0]["commanders"]["secondary"]["relics"], json!(null));
        assert_eq!(opponents[0]["commanders"]["secondary"]["armaments"], json!(null));
        assert_eq!(opponents[0]["npc"]["type"], json!(38));
        assert_eq!(opponents[0]["npc"]["b_type"], json!(1));
        assert_eq!(opponents[0]["npc"]["experience"], json!(6650));
        let loot = opponents[0]["npc"]["loot"].as_array().expect("npc loot array");
        assert_eq!(loot.len(), 3);
        assert_eq!(loot[0], json!({ "type": 2, "sub_type": 7005, "value": 66 }));
        assert_eq!(
            opponents[0]["battle_results"]["sender"],
            json!({
                "reinforcements_join": 0,
                "reinforcements_leave": 0,
                "kill_points": 0,
                "acclaim": 0,
                "severely_wounded": 10,
                "slightly_wounded": 842,
                "remaining": 199148,
                "dead": 0,
                "heal": 0,
                "troop_units": 200000,
                "troop_units_max": 200000,
                "watchtower_max": 0,
                "watchtower": 0,
                "power": -100,
                "attack_power": -100,
                "skill_power": 0,
                "merits": 0,
                "death_reduction": 0,
                "severe_wound_reduction": 0
            })
        );
        assert_eq!(
            opponents[0]["battle_results"]["opponent"],
            json!({
                "reinforcements_join": 0,
                "reinforcements_leave": 0,
                "kill_points": 0,
                "acclaim": 0,
                "severely_wounded": 0,
                "slightly_wounded": 0,
                "remaining": 0,
                "dead": 20247,
                "heal": 0,
                "troop_units": 195526,
                "troop_units_max": 210000,
                "watchtower_max": 0,
                "watchtower": 0,
                "power": -80988,
                "attack_power": -80988,
                "skill_power": 0,
                "merits": 0,
                "death_reduction": 0,
                "severe_wound_reduction": 0
            })
        );
        let participants = opponents[0]["participants"].as_array().expect("participants array");
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0]["participant_id"], json!(54797));
    }
}
