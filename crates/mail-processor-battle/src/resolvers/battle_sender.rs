use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};

use crate::context::MailContext;
use crate::helpers;
use crate::structures::{
    BattleAlliance, BattleArmament, BattleCastle, BattleCommander, BattleCommanders, BattleData,
    BattleMail, BattleParticipant, BattlePosition, BattleSubParticipant,
};

/// Resolves sender data for battle reports.
pub struct BattleSenderResolver;

impl Default for BattleSenderResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleSenderResolver {
    pub fn new() -> Self {
        Self
    }

    fn section_has_pname(section: &Value) -> bool {
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

    fn find_sender_section(sections: &[Value]) -> Option<(usize, &Map<String, Value>)> {
        for (index, section) in sections.iter().enumerate() {
            if section.get("PName").is_some() {
                return section.as_object().map(|map| (index, map));
            }
            if let Some(content) = section.pointer("/body/content").and_then(Value::as_object)
                && content.get("PName").is_some()
            {
                return Some((index, content));
            }
            if let Some(content) = section.get("content").and_then(Value::as_object)
                && content.get("PName").is_some()
            {
                return Some((index, content));
            }
        }

        None
    }

    fn find_sender_value<'a>(
        sections: &'a [Value],
        sender_index: usize,
        key: &str,
    ) -> Option<&'a Value> {
        let mut start = sender_index;
        while start > 0 && !Self::section_has_pname(&sections[start - 1]) {
            start -= 1;
        }

        let mut end = sender_index + 1;
        while end < sections.len() && !Self::section_has_pname(&sections[end]) {
            end += 1;
        }

        // Sender detail blocks can spill into surrounding sections without PName fields.
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

    fn find_self_char(sections: &[Value]) -> Option<&Map<String, Value>> {
        sections.iter().find_map(|section| {
            section
                .pointer("/body/content/SelfChar")
                .or_else(|| section.pointer("/content/SelfChar"))
                .and_then(Value::as_object)
        })
    }

    fn find_sts_map(sections: &[Value]) -> Option<&Map<String, Value>> {
        sections.iter().find_map(|section| {
            section
                .get("STs")
                .or_else(|| section.pointer("/body/STs"))
                .and_then(Value::as_object)
        })
    }

    fn parse_trimmed_string(value: Option<&Value>) -> Option<String> {
        let raw = helpers::parse_string(value?)?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    }

    fn parse_commander_field(value: Option<&Value>) -> Option<i64> {
        let value = helpers::parse_i64(value?)?;
        if value == 0 { None } else { Some(value) }
    }

    fn to_commander(
        id_value: Option<&Value>,
        level_value: Option<&Value>,
    ) -> Option<BattleCommander> {
        let id = Self::parse_commander_field(id_value);
        let level = Self::parse_commander_field(level_value);

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

    fn to_secondary_commander(
        id_value: Option<&Value>,
        level_value: Option<&Value>,
    ) -> Option<BattleCommander> {
        let id = Self::parse_commander_field(id_value)?;
        let level = Self::parse_commander_field(level_value);

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

    fn to_commanders(
        primary_id: Option<&Value>,
        primary_level: Option<&Value>,
        secondary_id: Option<&Value>,
        secondary_level: Option<&Value>,
    ) -> Option<BattleCommanders> {
        let primary = Self::to_commander(primary_id, primary_level);
        let secondary = Self::to_secondary_commander(secondary_id, secondary_level);

        if primary.is_none() && secondary.is_none() {
            return None;
        }

        Some(BattleCommanders { primary, secondary })
    }

    fn to_alliance(tag: Option<String>, name: Option<String>) -> Option<BattleAlliance> {
        if tag.is_none() && name.is_none() {
            return None;
        }

        Some(BattleAlliance { tag, name })
    }

    fn to_position(value: Option<&Value>) -> Option<BattlePosition> {
        let value = value.and_then(Value::as_object)?;
        let x = helpers::parse_position(value.get("X"));
        let y = helpers::parse_position(value.get("Y"));

        if x.is_none() && y.is_none() {
            None
        } else {
            Some(BattlePosition { x, y })
        }
    }

    fn to_armaments(hwbs: Option<&Value>) -> Option<Vec<BattleArmament>> {
        let hwbs = hwbs.and_then(Value::as_object)?;
        let mut items = Vec::new();
        for (key, entry) in hwbs {
            let Ok(id) = key.parse::<i64>() else {
                continue;
            };
            let entry = entry.as_object();
            let buffs = entry.and_then(|map| Self::parse_trimmed_string(map.get("Buffs")));
            let inscriptions = entry.and_then(|map| Self::parse_trimmed_string(map.get("Affix")));

            items.push(BattleArmament {
                slot: Some(id),
                buffs,
                inscriptions,
            });
        }

        if items.is_empty() { None } else { Some(items) }
    }

    fn build_sender(
        sections: &[Value],
        sender_map: Option<&Map<String, Value>>,
        sender_index: Option<usize>,
        self_char: Option<&Map<String, Value>>,
        participants: Vec<BattleSubParticipant>,
    ) -> Option<BattleParticipant> {
        let player_id = self_char
            .and_then(|map| map.get("PId").and_then(helpers::parse_i64))
            .or_else(|| sender_map.and_then(|map| map.get("PId").and_then(helpers::parse_i64)));

        let player_name = sender_map.and_then(|map| Self::parse_trimmed_string(map.get("PName")));
        let app_uid = sender_map.and_then(|map| Self::parse_trimmed_string(map.get("AppUid")));
        let camp = self_char
            .and_then(|map| map.get("SideId").and_then(helpers::parse_i64))
            .or_else(|| sender_map.and_then(|map| map.get("SideId").and_then(helpers::parse_i64)));
        let kingdom = sender_map.and_then(|map| map.get("COSId").and_then(helpers::parse_i64));

        let alliance_tag = sender_map
            .and_then(|map| Self::parse_trimmed_string(map.get("Abbr")))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "Abbr")
                        .and_then(|value| Self::parse_trimmed_string(Some(value)))
                })
            });
        let alliance_name = sender_map
            .and_then(|map| Self::parse_trimmed_string(map.get("AName")))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "AName")
                        .and_then(|value| Self::parse_trimmed_string(Some(value)))
                })
            });
        let alliance = Self::to_alliance(alliance_tag, alliance_name);

        let (avatar_url, frame_url) = helpers::parse_avatar(
            self_char
                .and_then(|map| map.get("Avatar"))
                .or_else(|| sender_map.and_then(|map| map.get("Avatar"))),
        );

        let primary_id = sender_map
            .and_then(|map| Self::parse_commander_field(map.get("HId")))
            .or_else(|| self_char.and_then(|map| Self::parse_commander_field(map.get("HId"))))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HId")
                        .and_then(|value| Self::parse_commander_field(Some(value)))
                })
            });
        let primary_level = sender_map
            .and_then(|map| Self::parse_commander_field(map.get("HLv")))
            .or_else(|| self_char.and_then(|map| Self::parse_commander_field(map.get("HLv"))))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HLv")
                        .and_then(|value| Self::parse_commander_field(Some(value)))
                })
            });
        let primary_equipment = sender_map
            .and_then(|map| Self::parse_trimmed_string(map.get("HEq")))
            .or_else(|| self_char.and_then(|map| Self::parse_trimmed_string(map.get("HEq"))))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HEq")
                        .and_then(|value| Self::parse_trimmed_string(Some(value)))
                })
            });
        let primary_formation = sender_map
            .and_then(|map| map.get("HFMs").and_then(helpers::parse_i64))
            .or_else(|| self_char.and_then(|map| map.get("HFMs").and_then(helpers::parse_i64)))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HFMs").and_then(helpers::parse_i64)
                })
            });
        let primary_star = sender_map
            .and_then(|map| map.get("HSt").and_then(helpers::parse_i64))
            .or_else(|| self_char.and_then(|map| map.get("HSt").and_then(helpers::parse_i64)))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HSt").and_then(helpers::parse_i64)
                })
            });
        let primary_awakened = sender_map
            .and_then(|map| map.get("HAw").and_then(helpers::parse_bool))
            .or_else(|| self_char.and_then(|map| map.get("HAw").and_then(helpers::parse_bool)))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HAw").and_then(helpers::parse_bool)
                })
            });
        let primary_armaments = sender_map
            .and_then(|map| map.get("HWBs"))
            .or_else(|| self_char.and_then(|map| map.get("HWBs")))
            .or_else(|| {
                sender_index.and_then(|index| Self::find_sender_value(sections, index, "HWBs"))
            });
        let primary_armaments = Self::to_armaments(primary_armaments);
        let secondary_id = sender_map
            .and_then(|map| Self::parse_commander_field(map.get("HId2")))
            .or_else(|| {
                sender_index.and_then(|index| {
                    Self::find_sender_value(sections, index, "HId2")
                        .and_then(|value| Self::parse_commander_field(Some(value)))
                })
            });
        let secondary_level = secondary_id.and_then(|_| {
            sender_map
                .and_then(|map| Self::parse_commander_field(map.get("HLv2")))
                .or_else(|| self_char.and_then(|map| Self::parse_commander_field(map.get("HLv2"))))
                .or_else(|| {
                    sender_index.and_then(|index| {
                        Self::find_sender_value(sections, index, "HLv2")
                            .and_then(|value| Self::parse_commander_field(Some(value)))
                    })
                })
        });
        let secondary_equipment = secondary_id.and_then(|_| {
            sender_map
                .and_then(|map| Self::parse_trimmed_string(map.get("HEq2")))
                .or_else(|| self_char.and_then(|map| Self::parse_trimmed_string(map.get("HEq2"))))
                .or_else(|| {
                    sender_index.and_then(|index| {
                        Self::find_sender_value(sections, index, "HEq2")
                            .and_then(|value| Self::parse_trimmed_string(Some(value)))
                    })
                })
        });
        let secondary_star = secondary_id.and_then(|_| {
            sender_map
                .and_then(|map| map.get("HSt2").and_then(helpers::parse_i64))
                .or_else(|| self_char.and_then(|map| map.get("HSt2").and_then(helpers::parse_i64)))
                .or_else(|| {
                    sender_index.and_then(|index| {
                        Self::find_sender_value(sections, index, "HSt2")
                            .and_then(helpers::parse_i64)
                    })
                })
        });
        let secondary_awakened = secondary_id.and_then(|_| {
            sender_map
                .and_then(|map| map.get("HAw2").and_then(helpers::parse_bool))
                .or_else(|| self_char.and_then(|map| map.get("HAw2").and_then(helpers::parse_bool)))
                .or_else(|| {
                    sender_index.and_then(|index| {
                        Self::find_sender_value(sections, index, "HAw2")
                            .and_then(helpers::parse_bool)
                    })
                })
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

        let castle_pos = sender_map
            .and_then(|map| map.get("CastlePos"))
            .or_else(|| self_char.and_then(|map| map.get("CastlePos")))
            .or_else(|| {
                sender_index.and_then(|index| Self::find_sender_value(sections, index, "CastlePos"))
            });
        let castle = {
            let pos = Self::to_position(castle_pos);
            let level = sender_map
                .and_then(|map| map.get("CastleLevel").and_then(helpers::parse_i64))
                .or_else(|| {
                    self_char.and_then(|map| map.get("CastleLevel").and_then(helpers::parse_i64))
                })
                .or_else(|| {
                    sender_index.and_then(|index| {
                        Self::find_sender_value(sections, index, "CastleLevel")
                            .and_then(helpers::parse_i64)
                    })
                });
            let watchtower = sender_map
                .and_then(|map| map.get("GtLevel").and_then(helpers::parse_i64))
                .or_else(|| {
                    self_char.and_then(|map| map.get("GtLevel").and_then(helpers::parse_i64))
                })
                .or_else(|| {
                    sender_index.and_then(|index| {
                        Self::find_sender_value(sections, index, "GtLevel")
                            .and_then(helpers::parse_i64)
                    })
                });

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

    fn build_participants(sts: Option<&Map<String, Value>>) -> (Vec<BattleSubParticipant>, i64) {
        let Some(sts) = sts else {
            return (Vec::new(), 0);
        };

        let mut keys: Vec<&String> = sts.keys().collect();
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
            let Some(entry) = sts.get(key).and_then(Value::as_object) else {
                continue;
            };

            let player_id = entry.get("PId").and_then(helpers::parse_i64);
            let player_name = Self::parse_trimmed_string(entry.get("PName"));
            let alliance_tag = Self::parse_trimmed_string(entry.get("Abbr"));
            let alliance = Self::to_alliance(alliance_tag, None);
            let commanders = Self::to_commanders(
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
}

impl Resolver<MailContext<'_>, BattleMail> for BattleSenderResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let (sender_index, sender_map) = Self::find_sender_section(ctx.sections)
            .map(|(index, map)| (Some(index), Some(map)))
            .unwrap_or((None, None));
        let self_char = Self::find_self_char(ctx.sections);
        let sts = Self::find_sts_map(ctx.sections);

        let (participants, skipped) = Self::build_participants(sts);
        let sender = Self::build_sender(
            ctx.sections,
            sender_map,
            sender_index,
            self_char,
            participants,
        );

        if sender.is_none() {
            return Ok(());
        }

        output.metadata.rokb_battle_data_sender_skipped_participants = output
            .metadata
            .rokb_battle_data_sender_skipped_participants
            .saturating_add(skipped);
        output.battle_data = Some(BattleData { sender });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BattleSenderResolver;
    use crate::context::MailContext;
    use crate::structures::BattleMail;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_data(sections: &[Value]) -> BattleMail {
        let ctx = MailContext::new(sections);
        let mut output = BattleMail::default();
        let resolver = BattleSenderResolver::new();

        resolver
            .resolve(&ctx, &mut output)
            .expect("resolve battle data");

        output
    }

    #[test]
    fn battle_sender_resolver_merges_sender_fields() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 10,
                            "SideId": 2,
                            "HId": 1,
                            "Avatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}"
                        }
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "AppUid": 999,
                "COSId": 123,
                "Abbr": "TAG",
                "AName": "Alliance",
                "HEq": "primary-eq",
                "HEq2": "secondary-eq",
                "HFMs": 3,
                "HSt": 5,
                "HSt2": 4,
                "HAw": true,
                "HAw2": 0,
                "HId2": 2,
                "HLv": 60,
                "HLv2": 50
            }),
            json!({
                "STs": {
                    "100": {
                        "PId": 10,
                        "PName": "StsSender",
                        "Abbr": "ST",
                        "HId": 1,
                        "HLv": 60,
                        "HId2": 2,
                        "HLv2": 50
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");
        let sender = data.sender.expect("sender");

        assert_eq!(sender.player_id, Some(10));
        assert_eq!(sender.player_name.as_deref(), Some("Sender"));
        assert_eq!(sender.app_uid.as_deref(), Some("999"));
        assert_eq!(sender.camp, Some(2));
        assert_eq!(sender.kingdom, Some(123));
        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.tag.as_deref()),
            Some("TAG")
        );
        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.name.as_deref()),
            Some("Alliance")
        );
        assert_eq!(sender.avatar_url.as_deref(), Some("http://a"));
        assert_eq!(sender.frame_url.as_deref(), Some("http://f"));
        let commanders = sender.commanders.expect("commanders");
        assert_eq!(commanders.primary.as_ref().and_then(|c| c.id), Some(1));
        assert_eq!(commanders.primary.as_ref().and_then(|c| c.level), Some(60));
        assert_eq!(
            commanders
                .primary
                .as_ref()
                .and_then(|c| c.equipment.as_deref()),
            Some("primary-eq")
        );
        assert_eq!(
            commanders.primary.as_ref().and_then(|c| c.formation),
            Some(3)
        );
        assert_eq!(commanders.primary.as_ref().and_then(|c| c.star), Some(5));
        assert_eq!(
            commanders.primary.as_ref().and_then(|c| c.awakened),
            Some(true)
        );
        assert_eq!(commanders.secondary.as_ref().and_then(|c| c.id), Some(2));
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.level),
            Some(50)
        );
        assert_eq!(
            commanders
                .secondary
                .as_ref()
                .and_then(|c| c.equipment.as_deref()),
            Some("secondary-eq")
        );
        assert_eq!(commanders.secondary.as_ref().and_then(|c| c.star), Some(4));
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.awakened),
            Some(false)
        );

        assert_eq!(sender.participants.len(), 1);
        let participant = &sender.participants[0];
        assert_eq!(participant.player_id, Some(10));
        assert_eq!(participant.player_name.as_deref(), Some("StsSender"));
        assert_eq!(
            participant.alliance.as_ref().and_then(|a| a.tag.as_deref()),
            Some("ST")
        );
    }

    #[test]
    fn battle_sender_resolver_reads_sender_from_body_content() {
        let sections = vec![json!({
            "body": {
                "content": {
                    "SelfChar": {
                        "PId": 11,
                        "HId": 4,
                        "Avatar": {
                            "avatar": "http://b",
                            "avatarFrame": "http://g"
                        }
                    },
                    "PName": "BodySender",
                    "AppUid": "abc",
                    "COSId": "777",
                    "Abbr": "BB",
                    "AName": "BodyAlliance",
                    "HLv": 50,
                    "HId2": 9,
                    "HLv2": 40
                }
            }
        })];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");
        let sender = data.sender.expect("sender");

        assert_eq!(sender.player_id, Some(11));
        assert_eq!(sender.player_name.as_deref(), Some("BodySender"));
        assert_eq!(sender.app_uid.as_deref(), Some("abc"));
        assert_eq!(sender.kingdom, Some(777));
        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.tag.as_deref()),
            Some("BB")
        );
        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.name.as_deref()),
            Some("BodyAlliance")
        );
        assert_eq!(sender.avatar_url.as_deref(), Some("http://b"));
        assert_eq!(sender.frame_url.as_deref(), Some("http://g"));
        let commanders = sender.commanders.expect("commanders");
        assert_eq!(commanders.primary.as_ref().and_then(|c| c.id), Some(4));
        assert_eq!(commanders.primary.as_ref().and_then(|c| c.level), Some(50));
        assert_eq!(commanders.secondary.as_ref().and_then(|c| c.id), Some(9));
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.level),
            Some(40)
        );
    }

    #[test]
    fn battle_sender_resolver_reads_castle_details() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 10,
                            "GtLevel": 12
                        }
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "CastleLevel": 25,
                "CastlePos": {
                    "X": 12.4,
                    "Y": "18.1"
                }
            }),
        ];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");
        let castle = sender.castle.expect("castle");
        let pos = castle.pos.expect("castle pos");

        assert_eq!(pos.x, Some(2));
        assert_eq!(pos.y, Some(3));
        assert_eq!(castle.level, Some(25));
        assert_eq!(castle.watchtower, Some(12));
    }

    #[test]
    fn battle_sender_resolver_omits_watchtower_when_missing() {
        let sections = vec![json!({
            "PName": "Sender",
            "CastleLevel": 25,
            "CastlePos": {
                "X": 6.0,
                "Y": 6.0
            }
        })];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");
        let castle = sender.castle.expect("castle");
        let serialized = serde_json::to_value(&castle).expect("serialize castle");

        assert!(castle.watchtower.is_none());
        assert!(serialized.get("watchtower").is_none());
    }

    #[test]
    fn battle_sender_resolver_reads_alliance_from_adjacent_sections() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "uid-1"
            }),
            json!({
                "Abbr": "TAG",
                "AName": "Alliance"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "ST",
                        "HId": 10,
                        "HLv": 60
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");

        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.tag.as_deref()),
            Some("TAG")
        );
        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.name.as_deref()),
            Some("Alliance")
        );

        assert_eq!(sender.participants.len(), 1);
        assert_eq!(
            sender.participants[0]
                .alliance
                .as_ref()
                .and_then(|a| a.tag.as_deref()),
            Some("ST")
        );
    }

    #[test]
    fn battle_sender_resolver_ignores_sts_for_commander_fields() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 42,
                            "HId": 10
                        }
                    }
                }
            }),
            json!({
                "HSt2": 4,
                "HLv2": 50,
                "HAw2": 1
            }),
            json!({
                "Schema": 1
            }),
            json!({
                "PName": "Sender",
                "HId": 10,
                "HId2": 11
            }),
            json!({
                "STs": {
                    "-2": {
                        "PId": 42,
                        "PName": "",
                        "HId": 0,
                        "HLv": 0,
                        "HId2": 0,
                        "HLv2": 0
                    },
                    "0": {
                        "PId": 42,
                        "PName": "Sender",
                        "HId": 10,
                        "HLv": 60,
                        "HId2": 11,
                        "HLv2": 10
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");
        let commanders = sender.commanders.expect("commanders");

        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.level),
            Some(50)
        );
        assert_eq!(commanders.secondary.as_ref().and_then(|c| c.star), Some(4));
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.awakened),
            Some(true)
        );
        assert_eq!(sender.participants.len(), 1);
        assert_eq!(
            output.metadata.rokb_battle_data_sender_skipped_participants,
            1
        );
    }

    #[test]
    fn battle_sender_resolver_reads_commander_extras_from_sender_block() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 77,
                            "HId": 21
                        }
                    }
                }
            }),
            json!({
                "HSt": 6,
                "HAw": true,
                "HFMs": 4,
                "HLv2": 55,
                "HSt2": 5,
                "HAw2": 0
            }),
            json!({
                "Schema": 2
            }),
            json!({
                "PName": "Sender",
                "HId": 21,
                "HLv": 60,
                "HId2": 22
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 77,
                        "PName": "Sender",
                        "HId": 21,
                        "HLv": 10,
                        "HId2": 22,
                        "HLv2": 12
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");
        let commanders = sender.commanders.expect("commanders");

        assert_eq!(commanders.primary.as_ref().and_then(|c| c.star), Some(6));
        assert_eq!(
            commanders.primary.as_ref().and_then(|c| c.awakened),
            Some(true)
        );
        assert_eq!(
            commanders.primary.as_ref().and_then(|c| c.formation),
            Some(4)
        );
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.level),
            Some(55)
        );
        assert_eq!(commanders.secondary.as_ref().and_then(|c| c.star), Some(5));
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.awakened),
            Some(false)
        );
    }

    #[test]
    fn battle_sender_resolver_reads_primary_armaments() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 77,
                            "HId": 21
                        }
                    }
                }
            }),
            json!({
                "HWBs": {
                    "1": {
                        "Buffs": "3001_0.010000;4001_0.020000",
                        "Affix": "17101"
                    },
                    "4": {
                        "Buffs": "5001_0.030000",
                        "Affix": " "
                    },
                    "bad": {
                        "Buffs": "1001_0.010000"
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "HId": 21,
                "HLv": 60,
                "HId2": 22
            }),
        ];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");
        let commanders = sender.commanders.expect("commanders");
        let primary = commanders.primary.expect("primary");
        let armaments = primary.armaments.expect("armaments");
        let slot1 = armaments.iter().find(|arm| arm.slot == Some(1));
        let slot4 = armaments.iter().find(|arm| arm.slot == Some(4));

        assert_eq!(armaments.len(), 2);
        let slot1 = slot1.expect("slot 1");
        let slot4 = slot4.expect("slot 4");
        assert_eq!(slot1.slot, Some(1));
        assert_eq!(slot1.buffs.as_deref(), Some("3001_0.010000;4001_0.020000"));
        assert_eq!(slot1.inscriptions.as_deref(), Some("17101"));
        assert_eq!(slot4.slot, Some(4));
        assert_eq!(slot4.buffs.as_deref(), Some("5001_0.030000"));
        assert!(slot4.inscriptions.is_none());
        assert!(
            commanders
                .secondary
                .as_ref()
                .and_then(|c| c.armaments.as_ref())
                .is_none()
        );
    }

    #[test]
    fn battle_sender_resolver_omits_secondary_when_hid2_zero() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 21,
                            "HId": 7
                        }
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "HId2": 0,
                "HLv2": 40
            }),
            json!({
                "STs": {
                    "10": {
                        "PId": 21,
                        "PName": "Sender",
                        "HId": 7,
                        "HLv": 60,
                        "HId2": 0,
                        "HLv2": 40
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");
        let sender = data.sender.expect("sender");
        let commanders = sender.commanders.expect("commanders");

        assert_eq!(commanders.primary.as_ref().and_then(|c| c.id), Some(7));
        assert!(commanders.secondary.is_none());
        assert_eq!(sender.participants.len(), 1);
        assert!(
            sender.participants[0]
                .commanders
                .as_ref()
                .and_then(|c| c.secondary.as_ref())
                .is_none()
        );
    }

    #[test]
    fn battle_sender_resolver_sorts_participants_and_omits_empty_alliance_name() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 50,
                            "SideId": 1,
                            "HId": 101
                        }
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "COSId": 1234,
                "Abbr": "TAG",
                "AName": "  "
            }),
            json!({
                "STs": {
                    "-2": {
                        "PId": 999,
                        "PName": "",
                        "HId": 0,
                        "HLv": 0,
                        "HId2": 0,
                        "HLv2": 0
                    },
                    "10": {
                        "PId": 2,
                        "PName": "Second",
                        "Abbr": "ZZZ",
                        "HId": 20,
                        "HLv": 40,
                        "HId2": 0,
                        "HLv2": 50
                    },
                    "2": {
                        "PId": 1,
                        "PName": "First",
                        "Abbr": "AAA",
                        "HId": 10,
                        "HLv": 60,
                        "HId2": 12,
                        "HLv2": 40
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let sender = output
            .battle_data
            .expect("battle data")
            .sender
            .expect("sender");

        assert_eq!(
            sender.alliance.as_ref().and_then(|a| a.tag.as_deref()),
            Some("TAG")
        );
        assert!(
            sender
                .alliance
                .as_ref()
                .and_then(|a| a.name.as_ref())
                .is_none()
        );

        assert_eq!(sender.participants.len(), 2);
        assert_eq!(sender.participants[0].player_id, Some(1));
        assert_eq!(sender.participants[1].player_id, Some(2));
        assert_eq!(
            output.metadata.rokb_battle_data_sender_skipped_participants,
            1
        );
        assert!(
            sender.participants[1]
                .commanders
                .as_ref()
                .and_then(|c| c.secondary.as_ref())
                .is_none()
        );
    }

    #[test]
    fn battle_sender_resolver_skips_when_no_sender_data() {
        let sections = vec![json!({
            "Schema": 1
        })];

        let output = resolve_data(&sections);
        assert!(output.battle_data.is_none());
    }
}
