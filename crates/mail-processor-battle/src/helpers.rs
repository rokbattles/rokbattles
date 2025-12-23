use serde_json::{Map, Value};

use crate::structures::{
    BattleAlliance, BattleArmament, BattleCastle, BattleCommander, BattleCommanders,
    BattleParticipant, BattlePosition, BattleSubParticipant,
};

pub(crate) fn parse_i128(v: &Value) -> Option<i128> {
    match v {
        Value::Number(n) => n
            .as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from)),
        Value::String(s) => s.trim().parse::<i128>().ok(),
        _ => None,
    }
}

pub(crate) fn parse_i64(v: &Value) -> Option<i64> {
    parse_i128(v).and_then(|n| i64::try_from(n).ok())
}

pub(crate) fn parse_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|x| x != 0),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn parse_avatar(value: Option<&Value>) -> (Option<String>, Option<String>) {
    let Some(value) = value else {
        return (None, None);
    };

    match value {
        Value::String(raw) => {
            if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
                return (
                    parsed
                        .get("avatar")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    parsed
                        .get("avatarFrame")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                );
            }

            let trimmed = raw.trim();
            if trimmed.is_empty() {
                (None, None)
            } else {
                (Some(trimmed.to_owned()), None)
            }
        }
        Value::Object(map) => (
            map.get("avatar").and_then(Value::as_str).map(str::to_owned),
            map.get("avatarFrame")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        _ => (None, None),
    }
}

pub(crate) fn parse_position(value: Option<&Value>) -> Option<i64> {
    let raw = match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }?;

    if !raw.is_finite() {
        return None;
    }

    let scaled = (raw / 6.0).round();
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return None;
    }

    Some(scaled as i64)
}

pub(crate) fn parse_trimmed_string(value: Option<&Value>) -> Option<String> {
    let raw = parse_string(value?)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

pub(crate) fn parse_commander_field(value: Option<&Value>) -> Option<i64> {
    let value = parse_i64(value?)?;
    if value == 0 { None } else { Some(value) }
}

pub(crate) fn to_commander(
    id_value: Option<&Value>,
    level_value: Option<&Value>,
) -> Option<BattleCommander> {
    let id = parse_commander_field(id_value);
    let level = parse_commander_field(level_value);

    if id.is_none() && level.is_none() {
        return None;
    }

    Some(BattleCommander {
        id,
        level,
        equipment: None,
        formation: None,
        star: None,
        awakened: None,
        armaments: None,
    })
}

pub(crate) fn to_secondary_commander(
    id_value: Option<&Value>,
    level_value: Option<&Value>,
) -> Option<BattleCommander> {
    let id = parse_commander_field(id_value)?;
    let level = parse_commander_field(level_value);

    Some(BattleCommander {
        id: Some(id),
        level,
        equipment: None,
        formation: None,
        star: None,
        awakened: None,
        armaments: None,
    })
}

pub(crate) fn to_commanders(
    primary_id: Option<&Value>,
    primary_level: Option<&Value>,
    secondary_id: Option<&Value>,
    secondary_level: Option<&Value>,
) -> Option<BattleCommanders> {
    let primary = to_commander(primary_id, primary_level);
    let secondary = to_secondary_commander(secondary_id, secondary_level);

    if primary.is_none() && secondary.is_none() {
        return None;
    }

    Some(BattleCommanders { primary, secondary })
}

pub(crate) fn to_alliance(tag: Option<String>, name: Option<String>) -> Option<BattleAlliance> {
    if tag.is_none() && name.is_none() {
        return None;
    }

    Some(BattleAlliance { tag, name })
}

pub(crate) fn to_position(value: Option<&Value>) -> Option<BattlePosition> {
    let value = value.and_then(Value::as_object)?;
    let x = parse_position(value.get("X"));
    let y = parse_position(value.get("Y"));

    if x.is_none() && y.is_none() {
        None
    } else {
        Some(BattlePosition { x, y })
    }
}

pub(crate) fn to_armaments(hwbs: Option<&Value>) -> Option<Vec<BattleArmament>> {
    let hwbs = hwbs.and_then(Value::as_object)?;
    let mut items = Vec::new();
    for (key, entry) in hwbs {
        let Ok(id) = key.parse::<i64>() else {
            continue;
        };
        let entry = entry.as_object();
        let buffs = entry.and_then(|map| parse_trimmed_string(map.get("Buffs")));
        let inscriptions = entry.and_then(|map| parse_trimmed_string(map.get("Affix")));

        items.push(BattleArmament {
            slot: Some(id),
            buffs,
            inscriptions,
        });
    }

    if items.is_empty() { None } else { Some(items) }
}

pub(crate) fn section_has_pname(section: &Value) -> bool {
    section.get("PName").is_some()
        || section
            .pointer("/body/content/PName")
            .and_then(Value::as_str)
            .map(|p| !p.trim().is_empty())
            .unwrap_or(false)
        || section
            .pointer("/content/PName")
            .and_then(Value::as_str)
            .map(|p| !p.trim().is_empty())
            .unwrap_or(false)
}

pub(crate) fn find_participant_sections(sections: &[Value]) -> Vec<(usize, &Map<String, Value>)> {
    let mut entries = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        if section.get("PName").is_some() {
            if let Some(map) = section.as_object() {
                entries.push((index, map));
            }
            continue;
        }
        if let Some(content) = section.pointer("/body/content").and_then(Value::as_object)
            && content.get("PName").is_some()
        {
            entries.push((index, content));
            continue;
        }
        if let Some(content) = section.get("content").and_then(Value::as_object)
            && content.get("PName").is_some()
        {
            entries.push((index, content));
        }
    }

    entries
}

pub(crate) fn participant_block_bounds(
    sections: &[Value],
    participant_index: usize,
) -> (usize, usize) {
    let mut start = participant_index;
    while start > 0 && !section_has_pname(&sections[start - 1]) {
        start -= 1;
    }

    let mut end = participant_index + 1;
    while end < sections.len() && !section_has_pname(&sections[end]) {
        end += 1;
    }

    (start, end)
}

pub(crate) fn find_participant_value<'a>(
    sections: &'a [Value],
    participant_index: usize,
    key: &str,
) -> Option<&'a Value> {
    let (start, end) = participant_block_bounds(sections, participant_index);

    // Participant detail blocks can spill into surrounding sections without PName fields.
    for section in &sections[start..end] {
        if let Some(value) = section.get(key) {
            return Some(value);
        }
        if let Some(value) = section
            .pointer("/body/content")
            .and_then(|body| body.get(key))
        {
            return Some(value);
        }
        if let Some(value) = section.get("content").and_then(|content| content.get(key)) {
            return Some(value);
        }
    }

    None
}

pub(crate) fn build_participants(
    participants_map: Option<&Map<String, Value>>,
) -> (Vec<BattleSubParticipant>, i64) {
    let Some(participants_map) = participants_map else {
        return (Vec::new(), 0);
    };

    let mut keys: Vec<&String> = participants_map.keys().collect();
    keys.sort_by(|a, b| match (a.parse::<i64>(), b.parse::<i64>()) {
        (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(_), Err(_)) => a.cmp(b),
    });

    let mut participants = Vec::new();
    let mut skipped = 0;

    for key in keys {
        if key == "-2" {
            // Reserved parent/summary entry; exclude from participants.
            skipped += 1;
            continue;
        }
        let Some(entry) = participants_map.get(key).and_then(Value::as_object) else {
            continue;
        };

        let player_id = entry.get("PId").and_then(parse_i64);
        let player_name = parse_trimmed_string(entry.get("PName"));
        let alliance_tag = parse_trimmed_string(entry.get("Abbr"));
        let alliance = to_alliance(alliance_tag, None);
        let commanders = to_commanders(
            entry.get("HId"),
            entry.get("HLv"),
            entry.get("HId2"),
            entry.get("HLv2"),
        );

        if player_id.is_none()
            && player_name.is_none()
            && alliance.is_none()
            && commanders.is_none()
        {
            continue;
        }

        participants.push(BattleSubParticipant {
            player_id,
            player_name,
            alliance,
            commanders,
        });
    }

    (participants, skipped)
}

pub(crate) fn find_cidt_map_by_pid(
    sections: &[Value],
    player_id: i64,
) -> Option<&Map<String, Value>> {
    for section in sections {
        if let Some(found) = find_cidt_map_in_value(section, player_id) {
            return Some(found);
        }
    }

    None
}

fn find_cidt_map_in_value(value: &Value, player_id: i64) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(map) => {
            if let Some(cidt) = map.get("CIdt").and_then(Value::as_object)
                && cidt.get("PId").and_then(parse_i64) == Some(player_id)
            {
                return Some(cidt);
            }

            for value in map.values() {
                if let Some(found) = find_cidt_map_in_value(value, player_id) {
                    return Some(found);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                if let Some(found) = find_cidt_map_in_value(value, player_id) {
                    return Some(found);
                }
            }
        }
        _ => {}
    }

    None
}

fn find_cidt_avatar(
    sections: &[Value],
    player_id: i64,
) -> Option<(Option<String>, Option<String>)> {
    let cidt = find_cidt_map_by_pid(sections, player_id)?;
    let (avatar_url, frame_url) = parse_avatar(cidt.get("Avatar"));
    if avatar_url.is_some() || frame_url.is_some() {
        Some((avatar_url, frame_url))
    } else {
        None
    }
}

pub(crate) fn find_cidt_map_in_block(
    sections: &[Value],
    participant_index: usize,
) -> Option<&Map<String, Value>> {
    let (start, end) = participant_block_bounds(sections, participant_index);
    for section in &sections[start..end] {
        if let Some(found) = find_any_cidt_map(section) {
            return Some(found);
        }
    }

    None
}

pub(crate) fn find_any_cidt_map(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(map) => find_any_cidt_map_in_object(map),
        Value::Array(values) => {
            for value in values {
                if let Some(found) = find_any_cidt_map(value) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn find_any_cidt_map_in_object(map: &Map<String, Value>) -> Option<&Map<String, Value>> {
    if let Some(cidt) = map.get("CIdt").and_then(Value::as_object) {
        return Some(cidt);
    }
    for value in map.values() {
        if let Some(found) = find_any_cidt_map(value) {
            return Some(found);
        }
    }

    None
}

pub(crate) fn build_participant(
    sections: &[Value],
    participant_map: Option<&Map<String, Value>>,
    participant_index: Option<usize>,
    self_char: Option<&Map<String, Value>>,
    participants: Vec<BattleSubParticipant>,
    participant_seed: Option<&Map<String, Value>>,
) -> Option<BattleParticipant> {
    let player_id = self_char
        .and_then(|map| map.get("PId").and_then(parse_i64))
        .or_else(|| participant_map.and_then(|map| map.get("PId").and_then(parse_i64)))
        .or_else(|| participant_seed.and_then(|map| map.get("PId").and_then(parse_i64)));

    let player_name = participant_map
        .and_then(|map| parse_trimmed_string(map.get("PName")))
        .or_else(|| participant_seed.and_then(|map| parse_trimmed_string(map.get("PName"))));
    let app_uid = participant_map.and_then(|map| parse_trimmed_string(map.get("AppUid")));
    let mut camp = self_char
        .and_then(|map| map.get("SideId").and_then(parse_i64))
        .or_else(|| participant_map.and_then(|map| map.get("SideId").and_then(parse_i64)))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "SideId").and_then(parse_i64)
            })
        })
        .or_else(|| participant_seed.and_then(|map| map.get("SideId").and_then(parse_i64)));
    if camp.is_none() {
        // SideId is commonly stored in CIdt snapshots for opponents.
        if let Some(player_id) = player_id
            && let Some(cidt) = find_cidt_map_by_pid(sections, player_id)
        {
            camp = cidt.get("SideId").and_then(parse_i64);
        }
    }
    let kingdom = participant_map.and_then(|map| map.get("COSId").and_then(parse_i64));

    let alliance_tag = participant_map
        .and_then(|map| parse_trimmed_string(map.get("Abbr")))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "Abbr")
                    .and_then(|value| parse_trimmed_string(Some(value)))
            })
        })
        .or_else(|| participant_seed.and_then(|map| parse_trimmed_string(map.get("Abbr"))));
    let alliance_name = participant_map
        .and_then(|map| parse_trimmed_string(map.get("AName")))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "AName")
                    .and_then(|value| parse_trimmed_string(Some(value)))
            })
        });
    let alliance = to_alliance(alliance_tag, alliance_name);

    let (mut avatar_url, mut frame_url) = parse_avatar(
        self_char
            .and_then(|map| map.get("Avatar"))
            .or_else(|| participant_map.and_then(|map| map.get("Avatar")))
            .or_else(|| participant_seed.and_then(|map| map.get("Avatar"))),
    );
    if avatar_url.is_none() && frame_url.is_none() {
        // Opponent avatars often live in CIdt attack snapshots, keyed by PId.
        if let Some(player_id) = player_id
            && let Some((avatar, frame)) = find_cidt_avatar(sections, player_id)
        {
            avatar_url = avatar;
            frame_url = frame;
        }
    }

    let primary_id = participant_map
        .and_then(|map| parse_commander_field(map.get("HId")))
        .or_else(|| self_char.and_then(|map| parse_commander_field(map.get("HId"))))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HId")
                    .and_then(|value| parse_commander_field(Some(value)))
            })
        })
        .or_else(|| participant_seed.and_then(|map| parse_commander_field(map.get("HId"))));
    let primary_level = participant_map
        .and_then(|map| parse_commander_field(map.get("HLv")))
        .or_else(|| self_char.and_then(|map| parse_commander_field(map.get("HLv"))))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HLv")
                    .and_then(|value| parse_commander_field(Some(value)))
            })
        })
        .or_else(|| participant_seed.and_then(|map| parse_commander_field(map.get("HLv"))));
    let primary_equipment = participant_map
        .and_then(|map| parse_trimmed_string(map.get("HEq")))
        .or_else(|| self_char.and_then(|map| parse_trimmed_string(map.get("HEq"))))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HEq")
                    .and_then(|value| parse_trimmed_string(Some(value)))
            })
        });
    let primary_formation = participant_map
        .and_then(|map| map.get("HFMs").and_then(parse_i64))
        .or_else(|| self_char.and_then(|map| map.get("HFMs").and_then(parse_i64)))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HFMs").and_then(parse_i64)
            })
        });
    let primary_star = participant_map
        .and_then(|map| map.get("HSt").and_then(parse_i64))
        .or_else(|| self_char.and_then(|map| map.get("HSt").and_then(parse_i64)))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HSt").and_then(parse_i64)
            })
        });
    let primary_awakened = participant_map
        .and_then(|map| map.get("HAw").and_then(parse_bool))
        .or_else(|| self_char.and_then(|map| map.get("HAw").and_then(parse_bool)))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HAw").and_then(parse_bool)
            })
        });
    let primary_armaments = participant_map
        .and_then(|map| map.get("HWBs"))
        .or_else(|| self_char.and_then(|map| map.get("HWBs")))
        .or_else(|| {
            participant_index.and_then(|index| find_participant_value(sections, index, "HWBs"))
        });
    let primary_armaments = to_armaments(primary_armaments);
    let secondary_id = participant_map
        .and_then(|map| parse_commander_field(map.get("HId2")))
        .or_else(|| {
            participant_index.and_then(|index| {
                find_participant_value(sections, index, "HId2")
                    .and_then(|value| parse_commander_field(Some(value)))
            })
        })
        .or_else(|| participant_seed.and_then(|map| parse_commander_field(map.get("HId2"))));
    let secondary_level = secondary_id.and_then(|_| {
        participant_map
            .and_then(|map| parse_commander_field(map.get("HLv2")))
            .or_else(|| self_char.and_then(|map| parse_commander_field(map.get("HLv2"))))
            .or_else(|| {
                participant_index.and_then(|index| {
                    find_participant_value(sections, index, "HLv2")
                        .and_then(|value| parse_commander_field(Some(value)))
                })
            })
            .or_else(|| participant_seed.and_then(|map| parse_commander_field(map.get("HLv2"))))
    });
    let secondary_equipment = secondary_id.and_then(|_| {
        participant_map
            .and_then(|map| parse_trimmed_string(map.get("HEq2")))
            .or_else(|| self_char.and_then(|map| parse_trimmed_string(map.get("HEq2"))))
            .or_else(|| {
                participant_index.and_then(|index| {
                    find_participant_value(sections, index, "HEq2")
                        .and_then(|value| parse_trimmed_string(Some(value)))
                })
            })
    });
    let secondary_star = secondary_id.and_then(|_| {
        participant_map
            .and_then(|map| map.get("HSt2").and_then(parse_i64))
            .or_else(|| self_char.and_then(|map| map.get("HSt2").and_then(parse_i64)))
            .or_else(|| {
                participant_index.and_then(|index| {
                    find_participant_value(sections, index, "HSt2").and_then(parse_i64)
                })
            })
    });
    let secondary_awakened = secondary_id.and_then(|_| {
        participant_map
            .and_then(|map| map.get("HAw2").and_then(parse_bool))
            .or_else(|| self_char.and_then(|map| map.get("HAw2").and_then(parse_bool)))
            .or_else(|| {
                participant_index.and_then(|index| {
                    find_participant_value(sections, index, "HAw2").and_then(parse_bool)
                })
            })
            .or_else(|| participant_seed.and_then(|map| map.get("HAw2").and_then(parse_bool)))
    });

    let primary = if primary_id.is_none()
        && primary_level.is_none()
        && primary_equipment.is_none()
        && primary_formation.is_none()
        && primary_star.is_none()
        && primary_awakened.is_none()
        && primary_armaments.is_none()
    {
        None
    } else {
        Some(BattleCommander {
            id: primary_id,
            level: primary_level,
            equipment: primary_equipment,
            formation: primary_formation,
            star: primary_star,
            awakened: primary_awakened,
            armaments: primary_armaments,
        })
    };
    let secondary = secondary_id.map(|id| BattleCommander {
        id: Some(id),
        level: secondary_level,
        equipment: secondary_equipment,
        formation: None,
        star: secondary_star,
        awakened: secondary_awakened,
        armaments: None,
    });
    let commanders = if primary.is_none() && secondary.is_none() {
        None
    } else {
        Some(BattleCommanders { primary, secondary })
    };

    let castle_pos = participant_map
        .and_then(|map| map.get("CastlePos"))
        .or_else(|| self_char.and_then(|map| map.get("CastlePos")))
        .or_else(|| {
            participant_index.and_then(|index| find_participant_value(sections, index, "CastlePos"))
        });
    let castle = {
        let pos = to_position(castle_pos);
        let level = participant_map
            .and_then(|map| map.get("CastleLevel").and_then(parse_i64))
            .or_else(|| self_char.and_then(|map| map.get("CastleLevel").and_then(parse_i64)))
            .or_else(|| {
                participant_index.and_then(|index| {
                    find_participant_value(sections, index, "CastleLevel").and_then(parse_i64)
                })
            });
        let mut watchtower = participant_map
            .and_then(|map| map.get("GtLevel").and_then(parse_i64))
            .or_else(|| self_char.and_then(|map| map.get("GtLevel").and_then(parse_i64)))
            .or_else(|| {
                participant_index.and_then(|index| {
                    find_participant_value(sections, index, "GtLevel").and_then(parse_i64)
                })
            })
            .or_else(|| participant_seed.and_then(|map| map.get("GtLevel").and_then(parse_i64)));
        if watchtower.is_none() {
            // CIdt snapshots sometimes carry opponent watchtower levels.
            if let Some(player_id) = player_id
                && let Some(cidt) = find_cidt_map_by_pid(sections, player_id)
            {
                watchtower = cidt.get("GtLevel").and_then(parse_i64);
            }
        }

        if pos.is_none() && level.is_none() && watchtower.is_none() {
            None
        } else {
            Some(BattleCastle {
                pos,
                level,
                watchtower,
            })
        }
    };

    if player_id.is_none()
        && player_name.is_none()
        && app_uid.is_none()
        && camp.is_none()
        && kingdom.is_none()
        && alliance.is_none()
        && avatar_url.is_none()
        && frame_url.is_none()
        && commanders.is_none()
        && castle.is_none()
        && participants.is_empty()
    {
        return None;
    }

    Some(BattleParticipant {
        player_id,
        player_name,
        app_uid,
        camp,
        kingdom,
        alliance,
        avatar_url,
        frame_url,
        commanders,
        castle,
        participants,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_participant, parse_avatar, parse_position};
    use serde_json::json;

    #[test]
    fn parse_avatar_reads_json_string_payload() {
        let value = json!("{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}");

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert_eq!(avatar_url.as_deref(), Some("http://a"));
        assert_eq!(frame_url.as_deref(), Some("http://f"));
    }

    #[test]
    fn parse_avatar_accepts_plain_url_string() {
        let value = json!("https://example.com/avatar.png");

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert_eq!(
            avatar_url.as_deref(),
            Some("https://example.com/avatar.png")
        );
        assert!(frame_url.is_none());
    }

    #[test]
    fn parse_avatar_ignores_empty_string() {
        let value = json!("   ");

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert!(avatar_url.is_none());
        assert!(frame_url.is_none());
    }

    #[test]
    fn parse_avatar_reads_object_payload() {
        let value = json!({
            "avatar": "http://b",
            "avatarFrame": "http://g"
        });

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert_eq!(avatar_url.as_deref(), Some("http://b"));
        assert_eq!(frame_url.as_deref(), Some("http://g"));
    }

    #[test]
    fn parse_avatar_skips_non_string_objects() {
        let value = json!(123);

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert!(avatar_url.is_none());
        assert!(frame_url.is_none());
    }

    #[test]
    fn parse_position_scales_and_rounds() {
        let value = json!(12.4);

        let parsed = parse_position(Some(&value));

        assert_eq!(parsed, Some(2));
    }

    #[test]
    fn parse_position_handles_string_numbers() {
        let value = json!("18.1");

        let parsed = parse_position(Some(&value));

        assert_eq!(parsed, Some(3));
    }

    #[test]
    fn build_participant_uses_seed_avatar() {
        let sections = vec![json!({
            "PName": "Opponent"
        })];
        let participant_map = sections[0].as_object().expect("participant map");
        let seed = json!({
            "PId": 7,
            "PName": "Opponent",
            "Avatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}"
        });
        let seed_map = seed.as_object().expect("seed map");

        let participant = build_participant(
            &sections,
            Some(participant_map),
            Some(0),
            None,
            Vec::new(),
            Some(seed_map),
        )
        .expect("participant");

        assert_eq!(participant.avatar_url.as_deref(), Some("http://a"));
        assert_eq!(participant.frame_url.as_deref(), Some("http://f"));
    }

    #[test]
    fn build_participant_falls_back_to_cidt_avatar() {
        let sections = vec![
            json!({
                "PName": "Opponent",
                "PId": 10
            }),
            json!({
                "Attacks": {
                    "1": {
                        "CIdt": {
                            "PId": 10,
                            "Avatar": "{\"avatar\":\"http://b\",\"avatarFrame\":\"http://g\"}"
                        }
                    }
                }
            }),
        ];
        let participant_map = sections[0].as_object().expect("participant map");

        let participant = build_participant(
            &sections,
            Some(participant_map),
            Some(0),
            None,
            Vec::new(),
            None,
        )
        .expect("participant");

        assert_eq!(participant.avatar_url.as_deref(), Some("http://b"));
        assert_eq!(participant.frame_url.as_deref(), Some("http://g"));
    }

    #[test]
    fn build_participant_falls_back_to_cidt_watchtower() {
        let sections = vec![
            json!({
                "PName": "Opponent",
                "PId": 22
            }),
            json!({
                "Attacks": {
                    "1": {
                        "CIdt": {
                            "PId": 22,
                            "GtLevel": 25
                        }
                    }
                }
            }),
        ];
        let participant_map = sections[0].as_object().expect("participant map");

        let participant = build_participant(
            &sections,
            Some(participant_map),
            Some(0),
            None,
            Vec::new(),
            None,
        )
        .expect("participant");

        let castle = participant.castle.expect("castle");
        assert_eq!(castle.watchtower, Some(25));
        assert!(castle.pos.is_none());
        assert!(castle.level.is_none());
    }

    #[test]
    fn build_participant_falls_back_to_cidt_camp() {
        let sections = vec![
            json!({
                "PName": "Opponent",
                "PId": 44
            }),
            json!({
                "Attacks": {
                    "1": {
                        "CIdt": {
                            "PId": 44,
                            "SideId": 6
                        }
                    }
                }
            }),
        ];
        let participant_map = sections[0].as_object().expect("participant map");

        let participant = build_participant(
            &sections,
            Some(participant_map),
            Some(0),
            None,
            Vec::new(),
            None,
        )
        .expect("participant");

        assert_eq!(participant.camp, Some(6));
    }
}
