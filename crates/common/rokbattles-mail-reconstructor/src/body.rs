use serde_json::{Map, Value, json};

use crate::{
    ReconstructionError,
    dynamic::decode_message,
    entity::{bytes, text},
    protobuf::fields,
    reconstructor::MailReconstructor,
    value::normalize_lua_table,
};

impl MailReconstructor {
    pub(crate) fn reconstruct_body(
        &self,
        mail_type: &str,
        body: &[u8],
        attack_bodies: &[&[u8]],
    ) -> Result<Value, ReconstructionError> {
        match mail_type {
            "Battle" => {
                let mut content: Value =
                    serde_json::from_slice(body).map_err(ReconstructionError::InvalidBodyJson)?;
                merge_attack_bodies(&mut content, attack_bodies, &self.schema)?;
                normalize_lua_table(&mut content);
                Ok(json!({ "content": content }))
            }
            "DuelBattle2" => {
                let detail = decode_message(body, "DuelMailReport", &self.schema.descriptors)?;
                Ok(json!({ "title": "", "subTitle": "", "kvs": {}, "detail": detail }))
            }
            "Rss" => {
                let content = decode_message(body, "MailRss", &self.schema.descriptors)?;
                Ok(json!({ "content": content }))
            }
            "BarCanyonKillBoss" => self.reconstruct_bar_canyon(body),
            "EventMemberLootReport" => self.reconstruct_event_member_loot(body),
            "System" => self.reconstruct_system(body),
            "Alliance" => self.reconstruct_alliance(body),
            _ => Err(ReconstructionError::UnsupportedMailType(mail_type.to_string())),
        }
    }

    fn reconstruct_bar_canyon(&self, body: &[u8]) -> Result<Value, ReconstructionError> {
        let decoded = decode_message(body, "EliteBarReportInfo", &self.schema.descriptors)?;
        let root = object(&decoded, "EliteBarReportInfo")?;
        let infos = format_report_infos(root.get("Infos"), true)?;
        Ok(json!({
            "content": {
                "pos": rename_position(root.get("Pos")),
                "npcType": root.get("NpcType").cloned().unwrap_or(Value::from(0)),
                "npcLevel": root.get("Level").cloned().unwrap_or(Value::from(1)),
                "eliteBarName": "",
                "infos": infos,
            }
        }))
    }

    fn reconstruct_event_member_loot(&self, body: &[u8]) -> Result<Value, ReconstructionError> {
        let decoded = decode_message(body, "EventMemeberLootInfo", &self.schema.descriptors)?;
        let root = object(&decoded, "EventMemeberLootInfo")?;
        let subtitle_param = root.get("SubTitleParam").and_then(Value::as_str).unwrap_or_default();
        let subtitle = gve_boss_subtitle(subtitle_param);
        Ok(json!({
            "content": {
                "title": root.get("Title").cloned().unwrap_or(Value::String(String::new())),
                "subTitle": subtitle,
                "contentTxt": root.get("Body").cloned().unwrap_or(Value::String(String::new())),
                "infos": format_report_infos(root.get("Infos"), false)?,
                "EventName": root.get("EventName").cloned().unwrap_or(Value::String(String::new())),
            }
        }))
    }

    fn reconstruct_system(&self, body: &[u8]) -> Result<Value, ReconstructionError> {
        let decoded = decode_message(body, "MailSys", &self.schema.descriptors)?;
        let root = object(&decoded, "MailSys")?;
        let sub_type = root.get("Type").cloned().unwrap_or(Value::from(0));
        let sub_param = root.get("Param").cloned().unwrap_or(Value::from(0));
        let kvs = parse_kvs(root.get("Kvs"))?;
        let mut output = Map::new();
        output.insert("subType".to_string(), sub_type.clone());
        output.insert("subParam".to_string(), sub_param.clone());
        output.insert("title".to_string(), Value::String(String::new()));
        output.insert("subTitle".to_string(), Value::String(String::new()));
        output.insert("content".to_string(), Value::String(String::new()));

        if sub_type.as_i64() == Some(11) {
            let kvs = object(&kvs, "MailSys.Kvs")?;
            let npc_type = kvs.get("npc_type").and_then(Value::as_i64).unwrap_or_default();
            let level = npc_type.saturating_sub(100).max(1);
            let tier = kvs.get("order").and_then(Value::as_i64).unwrap_or_default();
            let damage = kvs.get("damage_rate").and_then(Value::as_f64).unwrap_or_default();
            let position =
                kvs.get("pos").cloned().unwrap_or_else(|| json!({"X": 0, "Y": 0, "Z": 0}));
            output.insert("targetName".to_string(), Value::String(format!("Level{level}")));
            output.insert("position".to_string(), position.clone());
            output.insert(
                "content".to_string(),
                Value::String(barbarian_fort_content(
                    sub_param.as_i64().unwrap_or_default(),
                    level,
                    damage,
                    tier,
                    &position,
                )),
            );
        }

        Ok(Value::Object(output))
    }

    fn reconstruct_alliance(&self, body: &[u8]) -> Result<Value, ReconstructionError> {
        let decoded = decode_message(body, "MailSys", &self.schema.descriptors)?;
        let root = object(&decoded, "MailSys")?;
        Ok(json!({
            "type": root.get("Type").cloned().unwrap_or(Value::from(0)),
            "param": root.get("Param").cloned().unwrap_or(Value::from(0)),
            "kvs": parse_kvs(root.get("Kvs"))?,
        }))
    }

    pub(crate) fn reconstruct_attachments(
        &self,
        attachments: &[&[u8]],
    ) -> Result<Vec<Value>, ReconstructionError> {
        attachments
            .iter()
            .map(|attachment| {
                let decoded =
                    decode_message(attachment, "MailAttachment", &self.schema.descriptors)?;
                let root = object(&decoded, "MailAttachment")?;
                Ok(json!({
                    "id": root.get("Id").cloned().unwrap_or(Value::from(0)),
                    "status": root.get("Status").cloned().unwrap_or(Value::from(0)),
                    "loot": root.get("Data").cloned().unwrap_or(Value::Array(Vec::new())),
                }))
            })
            .collect()
    }
}

fn barbarian_fort_content(
    sub_param: i64,
    level: i64,
    damage: f64,
    tier: i64,
    position: &Value,
) -> String {
    let x = game_map_coordinate(position, "X");
    let y = game_map_coordinate(position, "Y");
    let location = format!("X:{x} Y:{y}");
    match sub_param {
        1 => format!(
            "Congratulations! The Level {level} barbarian fort at {location} has been destroyed by your mighty onslaught.\n\nYou dealt {damage}% of the total damage and as a result have received the following <color=#00980e>Tier {tier}</color> trophies:"
        ),
        3 => format!(
            "Congratulations! The Marauder Encampment at {location} has been destroyed by your mighty onslaught. You dealt {damage}% of the total damage and as a result have received the following <color=#00980e>Tier {tier}</color> trophies:"
        ),
        4 => format!(
            "Congratulations! The level {level} Motte at {location} has been destroyed by your mighty onslaught. You dealt {damage}% of the total damage and as a result received <color=#00980e>level {tier}</color> plunder:"
        ),
        _ => format!("You dealt {damage}% of the total damage and received Tier {tier} rewards."),
    }
}

fn game_map_coordinate(position: &Value, axis: &str) -> i64 {
    position
        .get(axis)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| (value / 6.0).round() as i64)
        .unwrap_or_default()
}

fn merge_attack_bodies(
    body: &mut Value,
    attacks: &[&[u8]],
    schema: &crate::artifact::MailSchema,
) -> Result<(), ReconstructionError> {
    if attacks.is_empty() {
        return Ok(());
    }
    let attack_map = body
        .get_mut("Attacks")
        .and_then(Value::as_object_mut)
        .ok_or(ReconstructionError::MissingAttacksObject)?;
    for encoded in attacks {
        let mut name = None;
        let mut encoded_body = None;
        for field in fields(encoded) {
            let field = field?;
            if field.number == schema.attack_name {
                name = Some(text(field.value)?);
            } else if field.number == schema.attack_body {
                encoded_body = Some(bytes(field.value)?);
            }
        }
        let name = name.ok_or(ReconstructionError::MissingField("MailReportAttack.Attack"))?;
        let encoded_body =
            encoded_body.ok_or(ReconstructionError::MissingField("MailReportAttack.Body"))?;
        let mut value =
            serde_json::from_slice(encoded_body).map_err(ReconstructionError::InvalidAttackJson)?;
        normalize_lua_table(&mut value);
        attack_map.insert(name.to_string(), value);
    }
    Ok(())
}

fn object<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<&'a Map<String, Value>, ReconstructionError> {
    value.as_object().ok_or(ReconstructionError::InvalidBodyShape(name))
}

fn parse_kvs(value: Option<&Value>) -> Result<Value, ReconstructionError> {
    let text = value.and_then(Value::as_str).unwrap_or_default();
    if text.is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let mut value = serde_json::from_str(text).map_err(ReconstructionError::InvalidBodyJson)?;
    normalize_lua_table(&mut value);
    Ok(value)
}

fn format_report_infos(
    value: Option<&Value>,
    include_damage: bool,
) -> Result<Vec<Value>, ReconstructionError> {
    let infos = value
        .and_then(Value::as_array)
        .ok_or(ReconstructionError::InvalidBodyShape("report Infos"))?;
    infos
        .iter()
        .enumerate()
        .map(|(index, info)| {
            let info = object(info, "report Info")?;
            let mut output = Map::new();
            output.insert(
                "idx".to_string(),
                Value::from(u64::try_from(index.saturating_add(1)).unwrap_or(u64::MAX)),
            );
            output.insert(
                "playerId".to_string(),
                info.get("PlayerId").cloned().unwrap_or(Value::from(0)),
            );
            output.insert(
                "name".to_string(),
                info.get("Name").cloned().unwrap_or(Value::String(String::new())),
            );
            output.insert(
                "avatar".to_string(),
                info.get("Avatar").cloned().unwrap_or(Value::String(String::new())),
            );
            output.insert(
                "loots".to_string(),
                info.get("Loots").cloned().unwrap_or(Value::Array(Vec::new())),
            );
            if include_damage {
                output.insert(
                    "damageRate".to_string(),
                    info.get("DamageRate").cloned().unwrap_or(Value::from(0)),
                );
                output.insert(
                    "order".to_string(),
                    info.get("Order").cloned().unwrap_or(Value::from(0)),
                );
            } else {
                output.insert("extLoots".to_string(), Value::Array(Vec::new()));
                output.insert("season".to_string(), Value::from(0));
            }
            Ok(Value::Object(output))
        })
        .collect()
}

fn rename_position(value: Option<&Value>) -> Value {
    let Some(position) = value.and_then(Value::as_object) else {
        return json!({ "x": 0, "y": 0 });
    };
    json!({
        "x": position.get("X").cloned().unwrap_or(Value::from(0)),
        "y": position.get("Y").cloned().unwrap_or(Value::from(0)),
    })
}

fn gve_boss_subtitle(parameter: &str) -> String {
    let index = parameter.parse::<u64>().ok().map(|value| value / 20).unwrap_or_default();
    let name = match index {
        1 => "Bladefist Andaal",
        2 => "Bearkeeper Lukor",
        3 => "Shield Chieftain Murdos",
        4 => "Voodoo Priest Pache",
        5 => "Solon Por",
        _ => "Unknown GVE boss",
    };
    format!("{name} Has Been Defeated")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::MailReconstructor;

    fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
        while value >= 0x80 {
            output.push((value as u8) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }

    fn push_varint(output: &mut Vec<u8>, number: u32, value: u64) {
        encode_varint(u64::from(number) << 3, output);
        encode_varint(value, output);
    }

    fn push_bytes(output: &mut Vec<u8>, number: u32, value: &[u8]) {
        encode_varint((u64::from(number) << 3) | 2, output);
        encode_varint(value.len() as u64, output);
        output.extend_from_slice(value);
    }

    fn push_fixed64(output: &mut Vec<u8>, number: u32, value: f64) {
        encode_varint((u64::from(number) << 3) | 1, output);
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_fixed32(output: &mut Vec<u8>, number: u32, value: f32) {
        encode_varint((u64::from(number) << 3) | 5, output);
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn field(reconstructor: &MailReconstructor, message: &str, name: &str) -> u32 {
        reconstructor
            .schema
            .descriptors
            .message(message)
            .expect("message descriptor")
            .fields
            .iter()
            .find(|field| field.name == name)
            .expect("field descriptor")
            .number
    }

    fn duel_body(reconstructor: &MailReconstructor) -> Vec<u8> {
        let mut body = Vec::new();
        push_varint(&mut body, field(reconstructor, "DuelMailReport", "FightId"), 1);
        body
    }

    fn bar_canyon_body(reconstructor: &MailReconstructor) -> Vec<u8> {
        let mut position = Vec::new();
        push_fixed32(&mut position, field(reconstructor, "PosInfo", "X"), 12.5);
        push_fixed32(&mut position, field(reconstructor, "PosInfo", "Y"), 24.5);
        let mut body = Vec::new();
        push_bytes(&mut body, field(reconstructor, "EliteBarReportInfo", "Pos"), &position);
        push_varint(&mut body, field(reconstructor, "EliteBarReportInfo", "NpcType"), 401_000_093);
        push_varint(&mut body, field(reconstructor, "EliteBarReportInfo", "Level"), 35);
        body
    }

    fn event_member_loot_body(reconstructor: &MailReconstructor) -> Vec<u8> {
        let mut body = Vec::new();
        push_bytes(&mut body, field(reconstructor, "EventMemeberLootInfo", "Title"), b"GVE report");
        push_bytes(&mut body, field(reconstructor, "EventMemeberLootInfo", "SubTitleParam"), b"20");
        push_bytes(&mut body, field(reconstructor, "EventMemeberLootInfo", "EventName"), b"GVE");
        body
    }

    fn rss_body(reconstructor: &MailReconstructor) -> Vec<u8> {
        let mut position = Vec::new();
        push_fixed32(&mut position, field(reconstructor, "PosInfo", "X"), 12.5);
        push_fixed32(&mut position, field(reconstructor, "PosInfo", "Y"), 24.5);
        let mut body = Vec::new();
        push_varint(&mut body, field(reconstructor, "MailRss", "ResType"), 1);
        push_fixed64(&mut body, field(reconstructor, "MailRss", "ResValue"), 100.5);
        push_varint(&mut body, field(reconstructor, "MailRss", "Level"), 7);
        push_varint(&mut body, field(reconstructor, "MailRss", "Time"), 1234);
        push_bytes(&mut body, field(reconstructor, "MailRss", "Pos"), &position);
        body
    }

    fn system_body(reconstructor: &MailReconstructor, kind: u64, parameter: u64) -> Vec<u8> {
        let kvs = if kind == 11 {
            br#"{"damage_rate":15,"npc_type":107,"order":3,"pos":{"X":10,"Y":20,"Z":0}}"#.as_slice()
        } else {
            b"{}".as_slice()
        };
        let mut body = Vec::new();
        push_varint(&mut body, field(reconstructor, "MailSys", "Type"), kind);
        push_varint(&mut body, field(reconstructor, "MailSys", "Param"), parameter);
        push_bytes(&mut body, field(reconstructor, "MailSys", "Kvs"), kvs);
        body
    }

    #[test]
    fn merges_split_attack_body() {
        let reconstructor = MailReconstructor::synthetic();
        let mut body = json!({"Attacks": {}});
        let mut attack = Vec::new();
        push_bytes(&mut attack, reconstructor.schema.attack_name, b"9001");
        push_bytes(&mut attack, reconstructor.schema.attack_body, br#"{"Troops":42}"#);

        merge_attack_bodies(&mut body, &[&attack], &reconstructor.schema)
            .expect("attack should merge");

        assert_eq!(body["Attacks"]["9001"]["Troops"], 42);
    }

    #[test]
    fn reconstructs_every_supported_body_adapter() {
        let reconstructor = MailReconstructor::synthetic();
        let cases = [
            ("DuelBattle2", duel_body(&reconstructor)),
            ("BarCanyonKillBoss", bar_canyon_body(&reconstructor)),
            ("EventMemberLootReport", event_member_loot_body(&reconstructor)),
            ("Rss", rss_body(&reconstructor)),
            ("System", system_body(&reconstructor, 11, 1)),
            ("System", system_body(&reconstructor, 29, 11)),
            ("Alliance", system_body(&reconstructor, 60, 0)),
            ("Alliance", system_body(&reconstructor, 61, 0)),
            ("Alliance", system_body(&reconstructor, 62, 0)),
            ("Alliance", system_body(&reconstructor, 57, 1)),
        ];

        for (mail_type, body) in cases {
            reconstructor
                .reconstruct_body(mail_type, &body, &[])
                .unwrap_or_else(|error| panic!("{mail_type} should reconstruct: {error}"));
        }
    }

    #[test]
    fn reconstructed_barbarian_fort_body_preserves_packet_content_values() {
        let reconstructor = MailReconstructor::synthetic();
        let body = reconstructor
            .reconstruct_body("System", &system_body(&reconstructor, 11, 1), &[])
            .expect("barbarian fort body should reconstruct");
        let input = json!({
            "id": "mail-1",
            "time": 1,
            "receiver": "player_1",
            "serverId": 1,
            "body": body,
            "attachments": [],
        });

        let processed = rokbattles_mail_processor_system_barbarianfort::process(&input)
            .expect("reconstructed barbarian fort should process");
        let processed = serde_json::to_value(processed).expect("processed mail should serialize");

        assert_eq!(
            processed["body"]["content"],
            json!({"percentage": 15.0, "tier": 3, "level": 7})
        );
    }

    #[test]
    fn parses_and_normalizes_embedded_kvs() {
        let kvs = Value::String(r#"{"items":[1,"first",2,"second"],"empty":{}}"#.to_string());

        assert_eq!(
            parse_kvs(Some(&kvs)).expect("Kvs should parse"),
            json!({"items": ["first", "second"], "empty": []})
        );
        assert_eq!(parse_kvs(None).expect("missing Kvs should be empty"), json!({}));
    }

    #[test]
    fn rejects_invalid_embedded_kvs() {
        let kvs = Value::String("{".to_string());

        assert!(matches!(parse_kvs(Some(&kvs)), Err(ReconstructionError::InvalidBodyJson(_))));
    }

    #[test]
    fn formats_report_infos_for_both_adapter_shapes() {
        let infos = json!([{
            "PlayerId": 7,
            "Name": "Governor",
            "DamageRate": 12.5,
            "Order": 2
        }]);

        let damage = format_report_infos(Some(&infos), true).expect("infos should format");
        assert_eq!(damage[0]["idx"], 1);
        assert_eq!(damage[0]["damageRate"], 12.5);
        assert_eq!(damage[0]["order"], 2);

        let loot = format_report_infos(Some(&infos), false).expect("infos should format");
        assert_eq!(loot[0]["extLoots"], json!([]));
        assert_eq!(loot[0]["season"], 0);
    }

    #[test]
    fn supplies_position_and_gve_subtitle_fallbacks() {
        assert_eq!(rename_position(None), json!({"x": 0, "y": 0}));
        assert_eq!(
            rename_position(Some(&json!({"X": 10, "Y": 20, "Z": 30}))),
            json!({"x": 10, "y": 20})
        );
        assert_eq!(gve_boss_subtitle("20"), "Bladefist Andaal Has Been Defeated");
        assert_eq!(gve_boss_subtitle("invalid"), "Unknown GVE boss Has Been Defeated");
    }
}
