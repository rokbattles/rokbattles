use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};

use crate::context::MailContext;
use crate::helpers;
use crate::structures::{BattleData, BattleMail};

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
}

impl Resolver<MailContext<'_>, BattleMail> for BattleSenderResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let participant_sections = helpers::find_participant_sections(ctx.sections);
        let (sender_index, sender_map) = participant_sections
            .first()
            .map(|(index, map)| (Some(*index), Some(*map)))
            .unwrap_or((None, None));
        let self_char = Self::find_self_char(ctx.sections);
        let sts = Self::find_sts_map(ctx.sections);

        let (participants, skipped) = helpers::build_participants(sts);
        let sender = helpers::build_participant(
            ctx.sections,
            sender_map,
            sender_index,
            self_char,
            participants,
            None,
        );

        if sender.is_none() {
            return Ok(());
        }

        output.metadata.rokb_battle_data_sender_skipped_participants = output
            .metadata
            .rokb_battle_data_sender_skipped_participants
            .saturating_add(skipped);
        if let Some(data) = output.battle_data.as_mut() {
            data.sender = sender;
        } else {
            output.battle_data = Some(BattleData {
                sender,
                opponents: Vec::new(),
            });
        }

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
