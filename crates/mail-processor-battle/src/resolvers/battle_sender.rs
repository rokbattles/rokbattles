use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};

use crate::context::MailContext;
use crate::helpers;
use crate::structures::{
    BattleAlliance, BattleCommander, BattleCommanders, BattleData, BattleMail, BattleParticipant,
    BattleSender,
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

    fn find_sender_map(sections: &[Value]) -> Option<&Map<String, Value>> {
        for section in sections {
            if section.get("PName").is_some() {
                return section.as_object();
            }
            if let Some(content) = section.pointer("/body/content").and_then(Value::as_object)
                && content.get("PName").is_some()
            {
                return Some(content);
            }
            if let Some(content) = section.get("content").and_then(Value::as_object)
                && content.get("PName").is_some()
            {
                return Some(content);
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

    fn find_sts_entry_by_pid(
        sts: Option<&Map<String, Value>>,
        player_id: i64,
    ) -> Option<&Map<String, Value>> {
        let sts = sts?;
        sts.values()
            .filter_map(Value::as_object)
            .find(|entry| entry.get("PId").and_then(helpers::parse_i64) == Some(player_id))
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

        Some(BattleCommander { id, level })
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

    fn to_commanders_from_parts(
        primary_id: Option<i64>,
        primary_level: Option<i64>,
        secondary_id: Option<i64>,
        secondary_level: Option<i64>,
    ) -> Option<BattleCommanders> {
        let primary = if primary_id.is_none() && primary_level.is_none() {
            None
        } else {
            Some(BattleCommander {
                id: primary_id,
                level: primary_level,
            })
        };
        let secondary = secondary_id.map(|id| BattleCommander {
            id: Some(id),
            level: secondary_level,
        });

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

    fn build_sender(
        sender_map: Option<&Map<String, Value>>,
        self_char: Option<&Map<String, Value>>,
        sts: Option<&Map<String, Value>>,
        participants: Vec<BattleParticipant>,
    ) -> Option<BattleSender> {
        let player_id = self_char
            .and_then(|map| map.get("PId").and_then(helpers::parse_i64))
            .or_else(|| sender_map.and_then(|map| map.get("PId").and_then(helpers::parse_i64)));

        let sender_sts = player_id.and_then(|pid| Self::find_sts_entry_by_pid(sts, pid));

        let player_name = sender_map
            .and_then(|map| Self::parse_trimmed_string(map.get("PName")))
            .or_else(|| sender_sts.and_then(|map| Self::parse_trimmed_string(map.get("PName"))));
        let app_uid = sender_map.and_then(|map| Self::parse_trimmed_string(map.get("AppUid")));
        let camp = self_char
            .and_then(|map| map.get("SideId").and_then(helpers::parse_i64))
            .or_else(|| sender_map.and_then(|map| map.get("SideId").and_then(helpers::parse_i64)));
        let kingdom = sender_map.and_then(|map| map.get("COSId").and_then(helpers::parse_i64));

        let alliance_tag = sender_map
            .and_then(|map| Self::parse_trimmed_string(map.get("Abbr")))
            .or_else(|| sender_sts.and_then(|map| Self::parse_trimmed_string(map.get("Abbr"))));
        let alliance_name = sender_map.and_then(|map| Self::parse_trimmed_string(map.get("AName")));
        let alliance = Self::to_alliance(alliance_tag, alliance_name);

        let (avatar_url, frame_url) = helpers::parse_avatar(
            self_char
                .and_then(|map| map.get("Avatar"))
                .or_else(|| sender_map.and_then(|map| map.get("Avatar"))),
        );

        let primary_id = sender_map
            .and_then(|map| Self::parse_commander_field(map.get("HId")))
            .or_else(|| self_char.and_then(|map| Self::parse_commander_field(map.get("HId"))))
            .or_else(|| sender_sts.and_then(|map| Self::parse_commander_field(map.get("HId"))));
        let primary_level = sender_map
            .and_then(|map| Self::parse_commander_field(map.get("HLv")))
            .or_else(|| sender_sts.and_then(|map| Self::parse_commander_field(map.get("HLv"))));
        let secondary_id = sender_map
            .and_then(|map| Self::parse_commander_field(map.get("HId2")))
            .or_else(|| sender_sts.and_then(|map| Self::parse_commander_field(map.get("HId2"))));
        let secondary_level = secondary_id.and_then(|_| {
            sender_map
                .and_then(|map| Self::parse_commander_field(map.get("HLv2")))
                .or_else(|| sender_sts.and_then(|map| Self::parse_commander_field(map.get("HLv2"))))
        });

        let commanders = Self::to_commanders_from_parts(
            primary_id,
            primary_level,
            secondary_id,
            secondary_level,
        );

        if player_id.is_none()
            && player_name.is_none()
            && app_uid.is_none()
            && camp.is_none()
            && kingdom.is_none()
            && alliance.is_none()
            && avatar_url.is_none()
            && frame_url.is_none()
            && commanders.is_none()
            && participants.is_empty()
        {
            return None;
        }

        Some(BattleSender {
            player_id,
            player_name,
            app_uid,
            camp,
            kingdom,
            alliance,
            avatar_url,
            frame_url,
            commanders,
            participants,
        })
    }

    fn build_participants(sts: Option<&Map<String, Value>>) -> (Vec<BattleParticipant>, i64) {
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

            participants.push(BattleParticipant {
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
        let sender_map = Self::find_sender_map(ctx.sections);
        let self_char = Self::find_self_char(ctx.sections);
        let sts = Self::find_sts_map(ctx.sections);

        let (participants, skipped) = Self::build_participants(sts);
        let sender = Self::build_sender(sender_map, self_char, sts, participants);

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
                "HId2": 2,
                "HLv": 60,
                "HLv2": 50
            }),
            json!({
                "STs": {
                    "100": {
                        "PId": 10,
                        "PName": "Sender",
                        "Abbr": "TAG",
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
        assert_eq!(commanders.secondary.as_ref().and_then(|c| c.id), Some(2));
        assert_eq!(
            commanders.secondary.as_ref().and_then(|c| c.level),
            Some(50)
        );

        assert_eq!(sender.participants.len(), 1);
        let participant = &sender.participants[0];
        assert_eq!(participant.player_id, Some(10));
        assert_eq!(participant.player_name.as_deref(), Some("Sender"));
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
