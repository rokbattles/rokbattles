//! Adapts scout report data for persistent mail.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

const HERO_SKIN_SLOT: i64 = 22;

pub(crate) fn scout_content(report: &Value) -> Value {
    let target = &report["Char"];
    let mut character = json!({
        "id": number(target, "Id"),
        "type": number(target, "Type"),
        "pos": position(&target["Pos"]),
    });
    if let Some(alliance) = target.get("Alliance") {
        insert_field(
            &mut character,
            "alliance",
            json!({
                "id": alliance["Id"],
                "name": alliance["Name"],
                "abbr": alliance["Abbr"],
                "logo": alliance["Logo"],
            }),
        );
    }

    let outpost_rallies = list(&report["RallyTroops"])
        .iter()
        .enumerate()
        .map(|(index, rally)| {
            let mut output = json!({
                "rallyTroop": troop(&rally["RallyTroop"]),
                "rallyHeros": heroes(&rally["RallyHeros"]),
            });
            if let Some(skills) = list(&report["RallySkills"]).get(index) {
                insert_field(
                    &mut output,
                    "rallySkills",
                    json!(scout_skills(&skills["RallyStripSkills"])),
                );
            }
            output
        })
        .collect::<Vec<_>>();

    let tower = &report["Tower"];
    let durability = &report["Durability"];
    let mut content = json!({
        "scoutPlayerId": number(report, "ScoutPlayerId"),
        "scoutPlayerName": report["Name"].as_str().unwrap_or_default(),
        "char": character,
        "playerId": number(report, "PlayerId"),
        "troop": troop(&report["Troop"]),
        "heros": heroes(&report["Heros"]),
        "reinforce": troop(&report["Reinforce"]),
        "tower": {
            "hp": number(tower, "HP"),
            "hpStamp": number(tower, "HP"),
            "hpSpeed": number(tower, "HPSpeed"),
            "interval": number(tower, "Interval"),
            "nextUpdateTime": number(tower, "NextUpdateTime"),
            "level": number(tower, "Level"),
        },
        "durability": {
            "state": number(durability, "State"),
            "max": number(durability, "Max"),
            "num": number(durability, "Num"),
            "numStamp": number(durability, "Num"),
            "speed": number(durability, "Speed"),
            "nextUpdateTime": number(durability, "NextUpdateTime"),
            "interval": number(durability, "Interval"),
            "repairCDEndTime": number(durability, "RepairCDEndTime"),
            "stateEndTime": number(durability, "StateEndTime"),
            "outfireCDEndTime": number(durability, "OutfireCDEndTime"),
        },
        "strongholdType": number(report, "StrongholdType"),
        "allianceBuildingType": number(report, "AllianceBuildingType"),
        "ress": map_list(&report["Ress"], |resource| json!({
            "type": resource["Type"],
            "value": resource["Value"],
        })),
        "playerTech": number(report, "TechLevel"),
        "battleTech": number(report, "Tech2Level"),
        "p": 1.0 + (report["P"].as_f64().unwrap_or_default() - 0.5) / 2.0,
        "isRally": flag(report, "IsRally"),
        "rallyTroop": troop(&report["RallyTroop"]),
        "rallyHeros": heroes(&report["RallyHeros"]),
        "avatar": report["Avatar"].as_str().unwrap_or_default(),
        "gopPhase": number(report, "GopPhase"),
        "nonBinAcc": flag(report, "NonBindAcc"),
        "kvkSkills": scout_skills(&report["CaptainStripSkills"]),
        "rallyKvkSkills": scout_skills(&report["RallyStripSkills"]),
        "isConquerSeason": report["LostLandType"].as_i64().unwrap_or_default() >= 4,
        "snatchCnt": number(report, "SnatchCnt"),
        "snatchLimit": number(report, "SnatchLimit"),
        "isTurret": flag(report, "IsTurret"),
        "isOutpost": flag(report, "IsOutpost"),
        "schema": number(report, "Schema"),
        "isSameSide": flag(report, "IsSameSide"),
        "reinforces": map_list(&report["Reinforces"], troop),
        "outpostRallyTroops": outpost_rallies,
        "reserveTroop": troop(&report["ReserveTroops"]),
    });
    if let Some(country) = report.get("CountryId") {
        insert_field(&mut content, "countryId", country.clone());
    }
    content
}

fn troop(source: &Value) -> Value {
    let slim_units = list(&source["SUnits"]);
    let units = if slim_units.is_empty() { list(&source["Units"]) } else { slim_units };
    let trust = &source["TrustInfo"];
    let buffs = &source["Buffs"];
    let recovery = &source["PaySkinRecover"];
    let camp_no = source["CampNo"].as_i64().unwrap_or_default();
    let mut output = json!({
        "troopId": number(source, "TroopId"),
        "tag": source["Tag"].as_str().unwrap_or_default(),
        "playerId": number(source, "PlayerId"),
        "heros": map_list(&source["Heros"], troop_hero),
        "heroLevel": number(source, "HeroLevel"),
        "playerName": source["PlayerName"].as_str().unwrap_or_default(),
        "playerAvatar": source["PlayerAvatar"].as_str().unwrap_or_default(),
        "transferLimit": number(source, "TransferLimit"),
        "transferMax": number(source, "TransferMax"),
        "transferUpdateTime": number(source, "TransferUpdateTime"),
        "loots": map_list(&source["Loots"], |loot| json!({
            "Type": loot["Type"],
            "SubType": loot["SubType"],
            "Value": loot["Value"],
        })),
        "stripSkills": troop_skills(&source["StripSkills"]),
        "apTarget": number(source, "APTarget"),
        "campNo": camp_no,
        "obstacleMode": number(source, "ObstacleMode"),
        "tactics": number(source, "Tactics"),
        "isAggressiveRallyCaptain": flag(source, "IsAggressiveRallyCaptain"),
        "isStop": flag(source, "IsStop"),
        "joinOrleans": flag(source, "JoinOrleans"),
        "inOrleans": flag(source, "InOrleans"),
        "troopMark": map_list(&source["TroopMark"], |mark| json!({
            "AllianceId": mark["AllianceId"],
            "Comment": mark["Comment"],
            "Mark": mark["Mark"],
            "ExpireTime": mark["ExpireTime"],
        })),
        "isCaptain": flag(source, "IsCaptain"),
        "abbr": source["Abbr"].as_str().unwrap_or_default(),
        "xbuffs": map_list(&buffs["Nodes"], |buff| json!({
            "Key": buff["Key"], "Op": buff["Op"], "Type": buff["Type"],
            "Vf": buff["Vf"], "Expire": buff["Expire"], "StartTs": buff["StartTs"],
        })),
        "trustInfo": {
            "trustShId": number(trust, "TrustShId"),
            "trustState": number(trust, "TrustState"),
            "isCaptain": flag(trust, "IsCaptain"),
            "arrivedTm": number(trust, "ArrivedTm"),
        },
        "units": units.iter()
            .filter(|unit| unit["Count"].as_i64().is_some_and(|count| count > 0))
            .map(|unit| json!({
                "unitId": unit["UnitId"],
                "unitCount": unit["Count"],
                "maxCount": unit["MaxCount"],
            }))
            .collect::<Vec<_>>(),
        "agInfo": army_group(&source["AgInfo"]),
        "paySkinRecover": {
            "recoverd": flag(recovery, "Recoverd"),
            "isRecovering": flag(recovery, "IsRecovering"),
            "startTick": number(recovery, "StartTick"),
            "totalTick": number(recovery, "TotalTick"),
        },
    });
    let camp = &source["ArchivePoint"];
    if camp_no != 0 && (nonzero(&camp["X"]) || nonzero(&camp["Y"])) {
        insert_field(&mut output, "campPos", position(camp));
    }
    output
}

fn army_group(group: &Value) -> Value {
    let tag = &group["Tag"];
    let formation = &group["Formation"];
    let archives = map_list(&formation["Archive"], |archive| {
        json!({
            "formationId": archive["Formation"],
            "warBookArchives": indexed_table(list(&archive["Archive"]).iter().filter_map(|book| {
                Some((book["Slot"].as_i64()?, json!({
                    "slot": book["Slot"],
                    "warBookId": book["WarBookId"],
                })))
            })),
        })
    });
    let mut output = json!({
        "isArmyReserve": flag(tag, "IsArmyReserve"),
        "armyGroupOwner": number(tag, "ArmyGroupOwner"),
        "armyGroupArrived": flag(tag, "ArmyGroupArrived"),
        "isMasterTroop": flag(tag, "IsMasterTroop"),
        "pos": position(&group["Pos"]),
        "formationInfo": {
            "formationId": number(formation, "Formation"),
            "heroWarBooks": indexed_table(list(&formation["WarBooks"]).iter().filter_map(|entry| {
                let book = &entry["WarBook"];
                let id = book["Id"].as_str().unwrap_or_default();
                let (cfg_id, order) = id.split_once('_').unwrap_or((id, ""));
                Some((entry["Slot"].as_i64()?, json!({
                    "slot": entry["Slot"],
                    "warBook": {
                        "id": id,
                        "cfgId": cfg_id.parse::<i64>().ok(),
                        "order": order.parse::<i64>().ok(),
                        "heroId": number(book, "HeroId"),
                        "buffs": map_list(&book["Buffs"], |buff| json!({
                            "type": buff["Type"],
                            "value": buff["Val"],
                        })),
                        "affixs": list(&book["Affixs"]),
                        "locked": flag(book, "Lock"),
                        "selected": flag(book, "IsSelected"),
                        "sortWeight": 0,
                        "lockAttr": [flag(book, "LockAttr0"), flag(book, "LockAttr1"), flag(book, "LockAttr2")],
                        "rerandomTimes": number(book, "RerandomTimes"),
                        "littleGuarantee": number(book, "LittleGuarantee"),
                        "bigGuarantee": number(book, "BigGuarantee"),
                    },
                })))
            })),
            "archives": archives,
        },
    });
    let equipment = group["Equipments"].as_str().unwrap_or_default();
    if let Some(equipment) = equipment.strip_prefix('{').and_then(|value| value.strip_suffix('}'))
        && !equipment.is_empty()
    {
        let equipments = json!(
            equipment
                .split(',')
                .filter_map(|item| {
                    let (slot, uid) = item.split_once(':')?;
                    let slot = slot.parse::<i64>().ok()?;
                    let id = uid.split('_').next().and_then(|id| id.parse::<i64>().ok());
                    Some(json!({ "Id": id, "Uid": uid, "Slot": slot }))
                })
                .collect::<Vec<_>>()
        );
        insert_field(&mut output, "equipments", equipments);
    }
    output
}

fn troop_hero(hero: &Value) -> Value {
    let skin_id = list(&hero["Skins"])
        .iter()
        .rev()
        .find(|skin| skin["K"].as_i64() == Some(HERO_SKIN_SLOT))
        .and_then(|skin| skin["V"].as_i64());
    let unit_display = match skin_id {
        Some(100201) => 5001,
        Some(100202) => 5002,
        Some(100203) => 5003,
        Some(100204) => 5004,
        _ => 0,
    };
    json!({
        "heroId": hero["HeroId"],
        "label": hero["Label"].as_i64().filter(|&label| label != 0).unwrap_or(1),
        "level": hero["Level"],
        "awaked": flag(hero, "Awaked"),
        "formationId": number(hero, "FormationId"),
        "skins": skins(&hero["Skins"]),
        "ezUnitShowId": unit_display,
    })
}

fn heroes(value: &Value) -> Vec<Value> {
    let mut heroes = map_list(value, |hero| {
        let mut output = json!({
            "heroId": hero["HeroId"],
            "label": hero["Label"].as_i64().filter(|&label| label != 0).unwrap_or(1),
            "level": hero["Level"],
            "star": hero["Star"],
            "awaked": flag(hero, "Awaked"),
            "stripedSkill": number(hero, "StripedSkill"),
            "skills": map_list(&hero["Skills"], |skill| json!({
                "idx": skill["Id"],
                "skillId": skill["SkillId"],
                "level": skill["Level"],
                "SkillTrans": map_list(&skill["SkillTrans"], |info| json!({
                    "Id": info["Id"],
                    "TransConfig": info["TransConfig"],
                    "RawEffectNum": info["RawEffectNum"],
                    "TransEffectNum": info["TransEffectNum"],
                    "EquipId": number(info, "EquipId"),
                })),
            })),
            "collections": list(&hero["Collections"]),
            "transJobId": number(hero, "TransJobId"),
            "transJobSkillInfos": map_list(&hero["SkillInfos"], |skill| json!({
                "OriginalSkillId": number(skill, "Id"),
                "SkillId": skill["SkillId"],
                "Level": skill["Level"],
                "transInfos": map_list(&skill["TransInfos"], |info| json!({
                    "Id": info["Id"],
                    "ConfigType": info["ConfigType"].as_i64().unwrap_or(1),
                    "RawEffectNum": info["RawEffectNum"].as_f64().unwrap_or(1.0),
                    "TransEffectNum": info["TransEffectNum"].as_f64().unwrap_or(1.0),
                })),
            })),
            "assistSkills": map_list(&hero["AssistSkill"], |skill| json!({
                "heroId": skill["HeroId"],
                "skillId": skill["SkillId"],
                "level": skill["Level"],
                "index": skill["Index"],
                "isLimited": flag(skill, "IsLimited"),
            })),
            "formationInfo": { "formationId": number(&hero["Formation"], "Formation") },
        });
        if !list(&hero["Skins"]).is_empty() {
            insert_field(&mut output, "skins", skins(&hero["Skins"]));
        }
        output
    });
    heroes.sort_by_key(|hero| hero["label"].as_i64());
    heroes
}

fn scout_skills(value: &Value) -> Vec<Value> {
    map_list(value, |skill| {
        json!({
            "skillId": skill["SkillId"],
            "heroId": skill["HeroId"],
            "level": skill["Level"],
            "banStripSkill": flag(skill, "BanStripSkill"),
        })
    })
}

fn troop_skills(value: &Value) -> Vec<Value> {
    list(value)
        .iter()
        .filter_map(|skill| {
            let id = ["CurSkillId", "SkillId"]
                .into_iter()
                .find_map(|field| skill[field].as_i64().filter(|&id| id > 0))?;
            Some(json!({ "id": id, "isBanned": flag(skill, "BanStripSkill") }))
        })
        .collect()
}

fn number(value: &Value, field: &str) -> Value {
    value.get(field).filter(|value| value.is_number()).cloned().unwrap_or_else(|| json!(0))
}

fn insert_field(object: &mut Value, field: &str, value: Value) {
    if let Some(object) = object.as_object_mut() {
        object.insert(field.to_owned(), value);
    }
}

fn flag(value: &Value, field: &str) -> bool {
    value[field].as_bool().unwrap_or_default()
}

fn nonzero(value: &Value) -> bool {
    value.as_f64().is_some_and(|number| number != 0.0)
}

fn position(value: &Value) -> Value {
    json!({ "x": number(value, "X"), "y": number(value, "Y") })
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

fn map_list(value: &Value, map: impl FnMut(&Value) -> Value) -> Vec<Value> {
    list(value).iter().map(map).collect()
}

fn skins(value: &Value) -> Value {
    indexed_table(
        list(value).iter().filter_map(|skin| Some((skin["K"].as_i64()?, skin["V"].clone()))),
    )
}

/// Uses an array for consecutive one-based keys and an object for sparse keys.
fn indexed_table(entries: impl Iterator<Item = (i64, Value)>) -> Value {
    let table = entries.collect::<BTreeMap<_, _>>();
    if table.keys().copied().eq(1..) {
        return Value::Array(table.into_values().collect());
    }
    Value::Object(
        table.into_iter().map(|(key, value)| (key.to_string(), value)).collect::<Map<_, _>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scout_preserves_rally_members_and_uses_positive_slim_units() {
        let report = json!({
            "Ress": [{"Type": 2, "Value": 50}],
            "Troop": {
                "Units": [{"UnitId": 10, "Count": 9}],
                "SUnits": [
                    {"UnitId": 11, "Count": 0, "MaxCount": 12},
                    {"UnitId": 12, "Count": 3, "MaxCount": 12},
                ],
            },
            "RallyTroops": [
                {"RallyTroop": {"TroopId": 41}},
                {"RallyTroop": {"TroopId": 42}},
            ],
            "RallySkills": [{"RallyStripSkills": [{"HeroId": 7, "SkillId": 8, "Level": 2}]}],
        });

        let body = scout_content(&report);
        assert_eq!(body["ress"], json!([{"type": 2, "value": 50}]));
        assert_eq!(body["troop"]["units"], json!([{"unitId": 12, "unitCount": 3, "maxCount": 12}]));
        let rallies = &body["outpostRallyTroops"];
        assert_eq!(rallies[0]["rallyTroop"]["troopId"], 41);
        assert_eq!(
            rallies[0]["rallySkills"],
            json!([{
                "heroId": 7,
                "skillId": 8,
                "level": 2,
                "banStripSkill": false
            }])
        );
        assert_eq!(rallies[1]["rallyTroop"]["troopId"], 42);
        assert!(rallies[1].get("rallySkills").is_none());
    }
}
