use serde_json::{Map, Value};

use crate::structures::{DuelBattle2Buff, DuelBattle2Commander, DuelBattle2Skill};

pub(crate) struct PlayerLocation<'a> {
    pub(crate) player: &'a Value,
    pub(crate) parent: &'a Value,
    pub(crate) section_index: usize,
}

pub(crate) struct DetailContext<'a> {
    pub(crate) attacker: Option<PlayerLocation<'a>>,
    pub(crate) defender: Option<PlayerLocation<'a>>,
}

impl<'a> DetailContext<'a> {
    pub(crate) fn def_index(&self) -> Option<usize> {
        self.defender.as_ref().map(|loc| loc.section_index)
    }
}

pub(crate) fn find_detail_context<'a>(sections: &'a [Value]) -> DetailContext<'a> {
    let attacker = find_player_location(sections, "AtkPlayer");
    let defender = find_player_location(sections, "DefPlayer");

    DetailContext { attacker, defender }
}

pub(crate) fn split_sections(
    sections: &[Value],
    defender_index: Option<usize>,
) -> (&[Value], &[Value]) {
    let split_at = defender_index.unwrap_or(sections.len());
    sections.split_at(split_at)
}

fn find_player_location<'a>(sections: &'a [Value], key: &str) -> Option<PlayerLocation<'a>> {
    for (idx, section) in sections.iter().enumerate() {
        if let Some(player) = section.get(key) {
            return Some(PlayerLocation {
                player,
                parent: section,
                section_index: idx,
            });
        }

        if let Some(detail) = section.get("body").and_then(|b| b.get("detail"))
            && let Some(player) = detail.get(key)
        {
            return Some(PlayerLocation {
                player,
                parent: detail,
                section_index: idx,
            });
        }
    }

    None
}

pub(crate) fn parse_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok())),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

pub(crate) fn parse_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|i| i != 0),
        Value::String(s) => match s.trim() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn parse_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

pub(crate) fn parse_avatar(value: &Value) -> (Option<String>, Option<String>) {
    let Value::String(raw) = value else {
        return (None, None);
    };

    let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
        return (None, None);
    };

    let avatar = parsed
        .get("avatar")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let frame = parsed
        .get("avatarFrame")
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    (avatar, frame)
}

pub(crate) fn find_first_section_with_key<'a>(
    sections: &'a [Value],
    key: &str,
) -> Option<&'a Value> {
    sections.iter().find(|section| section.get(key).is_some())
}

pub(crate) fn collect_buffs(sections: &[Value]) -> Vec<DuelBattle2Buff> {
    let mut buffs = Vec::new();

    for section in sections {
        let Some(obj) = section.as_object() else {
            continue;
        };

        if let Some(inner) = obj.get("Buffs")
            && let Some(buff) = buff_from_value(inner)
        {
            buffs.push(buff);
        }

        if let Some(buff) = buff_from_object(obj) {
            buffs.push(buff);
        }
    }

    buffs
}

pub(crate) fn build_commander(value: &Value) -> DuelBattle2Commander {
    let mut commander = DuelBattle2Commander::default();
    let Some(obj) = value.as_object() else {
        return commander;
    };

    commander.id = obj.get("HeroId").and_then(parse_i64);
    commander.level = obj.get("HeroLevel").and_then(parse_i64);
    commander.star = obj.get("Star").and_then(parse_i64);
    commander.awakened = obj.get("Awaked").and_then(parse_bool);

    commander
}

pub(crate) fn push_skill_from_value(value: &Value, skills: &mut Vec<DuelBattle2Skill>) {
    let Some(obj) = value.as_object() else {
        return;
    };

    let skill_id = obj.get("SkillId").and_then(parse_i64);
    let level = obj.get("Level").and_then(parse_i64);
    let order = obj.get("Id").and_then(parse_i64);

    if skill_id.is_none() && level.is_none() && order.is_none() {
        return;
    }

    skills.push(DuelBattle2Skill {
        id: skill_id,
        level,
        order,
    });
}

pub(crate) fn is_skill_only_object(obj: &Map<String, Value>) -> bool {
    obj.keys()
        .all(|k| matches!(k.as_str(), "Id" | "Level" | "SkillId" | "SkillTrans"))
}

fn buff_from_value(value: &Value) -> Option<DuelBattle2Buff> {
    let obj = value.as_object()?;
    buff_from_object(obj)
}

fn buff_from_object(obj: &Map<String, Value>) -> Option<DuelBattle2Buff> {
    if !obj.contains_key("BuffId") && !obj.contains_key("BuffValue") {
        return None;
    }

    Some(DuelBattle2Buff {
        id: obj.get("BuffId").and_then(parse_i64),
        value: obj.get("BuffValue").and_then(parse_f64),
    })
}
