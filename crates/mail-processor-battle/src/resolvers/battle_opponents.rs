use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};
use std::collections::HashSet;

use crate::context::MailContext;
use crate::helpers;
use crate::structures::{BattleData, BattleMail};

/// Resolves opponent data for battle reports.
pub struct BattleOpponentsResolver;

type AttackSeed<'a> = (
    &'a Map<String, Value>,
    Option<&'a Map<String, Value>>,
    Option<&'a Map<String, Value>>,
);
type OtsSeed<'a> = (&'a Map<String, Value>, Option<&'a Map<String, Value>>);

impl Default for BattleOpponentsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleOpponentsResolver {
    pub fn new() -> Self {
        Self
    }

    fn find_cidt_seed<'a>(
        sections: &'a [Value],
        opponent: &Map<String, Value>,
        participants: &[crate::structures::BattleSubParticipant],
        participant_index: Option<usize>,
    ) -> Option<&'a Map<String, Value>> {
        if let Some(pid) = opponent.get("PId").and_then(helpers::parse_i64) {
            return helpers::find_cidt_map_by_pid(sections, pid);
        }

        if let Some(name) = helpers::parse_trimmed_string(opponent.get("PName"))
            && let Some(pid) = participants
                .iter()
                .find(|participant| participant.player_name.as_deref() == Some(name.as_str()))
                .and_then(|participant| participant.player_id)
        {
            return helpers::find_cidt_map_by_pid(sections, pid);
        }

        if participants.len() == 1
            && let Some(pid) = participants[0].player_id
        {
            return helpers::find_cidt_map_by_pid(sections, pid);
        }

        if let Some(index) = participant_index {
            return helpers::find_cidt_map_in_block(sections, index);
        }

        None
    }

    fn collect_attack_seeds<'a>(sections: &'a [Value]) -> Vec<AttackSeed<'a>> {
        let mut seeds = Vec::new();

        for section in sections {
            let Some(attacks) = section.get("Attacks").and_then(Value::as_object) else {
                continue;
            };
            let pname = helpers::parse_trimmed_string(attacks.get("PName"));
            if pname.is_none() {
                continue;
            }
            let ots = section
                .get("OTs")
                .or_else(|| section.pointer("/body/OTs"))
                .and_then(Value::as_object)
                .or_else(|| attacks.get("OTs").and_then(Value::as_object));
            let cidt = Self::find_attack_cidt(attacks);
            seeds.push((attacks, ots, cidt));
        }

        seeds
    }

    fn find_attack_cidt(attacks: &Map<String, Value>) -> Option<&Map<String, Value>> {
        for value in attacks.values() {
            if let Some(obj) = value.as_object()
                && let Some(cidt) = obj.get("CIdt").and_then(Value::as_object)
            {
                return Some(cidt);
            }
        }

        None
    }

    fn collect_ots_maps<'a>(sections: &'a [Value]) -> Vec<OtsSeed<'a>> {
        let mut maps = Vec::new();

        for section in sections {
            if let Some(ots) = section
                .get("OTs")
                .or_else(|| section.pointer("/body/OTs"))
                .and_then(Value::as_object)
            {
                let cidt = helpers::find_any_cidt_map(section);
                maps.push((ots, cidt));
            }

            if let Some(attacks) = section.get("Attacks").and_then(Value::as_object) {
                if let Some(ots) = attacks.get("OTs").and_then(Value::as_object) {
                    let cidt = helpers::find_any_cidt_map_in_object(attacks);
                    maps.push((ots, cidt));
                }

                for attack in attacks.values() {
                    if let Some(obj) = attack.as_object()
                        && let Some(ots) = obj.get("OTs").and_then(Value::as_object)
                    {
                        let cidt = helpers::find_any_cidt_map_in_object(obj);
                        maps.push((ots, cidt));
                    }
                }
            }
        }

        maps
    }

    fn find_ots_map(
        sections: &[Value],
        participant_index: usize,
        ct_id: Option<i64>,
    ) -> Option<&Map<String, Value>> {
        let (start, end) = helpers::participant_block_bounds(sections, participant_index);
        let ct_key = ct_id.map(|id| id.to_string());
        let mut fallback: Option<&Map<String, Value>> = None;

        // Prefer OTs blocks in the same participant block; fall back to matching CtId elsewhere.
        for section in &sections[start..end] {
            let ots = section
                .get("OTs")
                .or_else(|| section.pointer("/body/OTs"))
                .and_then(Value::as_object);
            let Some(ots) = ots else {
                continue;
            };

            if let Some(key) = ct_key.as_ref()
                && ots.contains_key(key)
            {
                return Some(ots);
            }

            if fallback.is_none() {
                fallback = Some(ots);
            }
        }

        if fallback.is_some() {
            return fallback;
        }

        let ct_key = ct_key?;

        sections.iter().find_map(|section| {
            section
                .get("OTs")
                .or_else(|| section.pointer("/body/OTs"))
                .and_then(Value::as_object)
                .filter(|ots| ots.contains_key(&ct_key))
        })
    }
}

impl Resolver<MailContext<'_>, BattleMail> for BattleOpponentsResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let participant_sections = helpers::find_participant_sections(ctx.sections);
        if participant_sections.len() <= 1 {
            let attack_seeds = Self::collect_attack_seeds(ctx.sections);
            if !attack_seeds.is_empty() {
                let mut opponents = Vec::new();
                let mut skipped_total: i64 = 0;
                let mut seen = HashSet::new();

                // Some reports store the opponent in the Attacks block without a PName section.
                for (seed, ots, cidt) in attack_seeds {
                    let (participants, skipped) = helpers::build_participants(ots);
                    skipped_total = skipped_total.saturating_add(skipped);
                    let player_id = seed.get("PId").and_then(helpers::parse_i64);
                    let app_uid = helpers::parse_trimmed_string(seed.get("AppUid"));
                    let player_name = helpers::parse_trimmed_string(seed.get("PName"));
                    if player_id.is_none() && app_uid.is_none() && player_name.is_none() {
                        continue;
                    }

                    let dedupe_key = if let Some(pid) = player_id {
                        format!("pid:{pid}")
                    } else if let Some(app_uid) = app_uid.as_ref() {
                        format!("app:{app_uid}")
                    } else if let Some(name) = player_name.as_ref() {
                        format!("name:{name}")
                    } else {
                        "attack".to_string()
                    };
                    if !seen.insert(dedupe_key) {
                        continue;
                    }

                    let opponent = helpers::build_participant(
                        ctx.sections,
                        Some(seed),
                        None,
                        None,
                        participants,
                        // CIdt snapshots often carry PId/avatar when the attack seed does not.
                        cidt,
                    );
                    if let Some(opponent) = opponent {
                        opponents.push(opponent);
                    }
                }

                if opponents.is_empty() {
                    if skipped_total > 0 {
                        output
                            .metadata
                            .rokb_battle_data_opponents_skipped_participants = output
                            .metadata
                            .rokb_battle_data_opponents_skipped_participants
                            .saturating_add(skipped_total);
                    }
                    return Ok(());
                }

                output
                    .metadata
                    .rokb_battle_data_opponents_skipped_participants = output
                    .metadata
                    .rokb_battle_data_opponents_skipped_participants
                    .saturating_add(skipped_total);
                if let Some(data) = output.battle_data.as_mut() {
                    data.opponents = opponents;
                } else {
                    output.battle_data = Some(BattleData {
                        sender: None,
                        opponents,
                    });
                }

                return Ok(());
            }

            let ots_maps = Self::collect_ots_maps(ctx.sections);
            if ots_maps.is_empty() {
                return Ok(());
            }

            let mut opponents = Vec::new();
            let mut skipped_total: i64 = 0;
            let mut seen = HashSet::new();

            // Some reports omit opponent data outside OTs; keep OTs for participants only.
            for (ots, section_cidt) in ots_maps {
                let (participants, skipped) = helpers::build_participants(Some(ots));
                skipped_total = skipped_total.saturating_add(skipped);
                if participants.is_empty() {
                    continue;
                }

                let mut cidt_seed = participants
                    .first()
                    .and_then(|participant| participant.player_id)
                    .and_then(|pid| helpers::find_cidt_map_by_pid(ctx.sections, pid));
                if cidt_seed.is_none() && participants.len() <= 1 {
                    cidt_seed = section_cidt;
                }
                let dedupe_key = participants
                    .first()
                    .and_then(|participant| participant.player_id)
                    .map(|pid| format!("pid:{pid}"))
                    .unwrap_or_else(|| format!("idx:{}", opponents.len()));
                if !seen.insert(dedupe_key) {
                    continue;
                }

                let opponent = helpers::build_participant(
                    ctx.sections,
                    None,
                    None,
                    None,
                    participants,
                    cidt_seed,
                );
                if let Some(opponent) = opponent {
                    opponents.push(opponent);
                }
            }

            if opponents.is_empty() {
                if skipped_total > 0 {
                    output
                        .metadata
                        .rokb_battle_data_opponents_skipped_participants = output
                        .metadata
                        .rokb_battle_data_opponents_skipped_participants
                        .saturating_add(skipped_total);
                }
                return Ok(());
            }

            output
                .metadata
                .rokb_battle_data_opponents_skipped_participants = output
                .metadata
                .rokb_battle_data_opponents_skipped_participants
                .saturating_add(skipped_total);
            if let Some(data) = output.battle_data.as_mut() {
                data.opponents = opponents;
            } else {
                output.battle_data = Some(BattleData {
                    sender: None,
                    opponents,
                });
            }

            return Ok(());
        }

        let mut opponents = Vec::new();
        let mut skipped_total: i64 = 0;
        for (index, map) in participant_sections.iter().skip(1) {
            let ct_id = map.get("CtId").and_then(helpers::parse_i64);
            let ots = Self::find_ots_map(ctx.sections, *index, ct_id);
            let (participants, skipped) = helpers::build_participants(ots);
            skipped_total = skipped_total.saturating_add(skipped);
            let cidt_seed = Self::find_cidt_seed(ctx.sections, map, &participants, Some(*index));
            let opponent = helpers::build_participant(
                ctx.sections,
                Some(*map),
                Some(*index),
                None,
                participants,
                cidt_seed,
            );
            if let Some(opponent) = opponent {
                opponents.push(opponent);
            }
        }

        if opponents.is_empty() {
            if skipped_total > 0 {
                output
                    .metadata
                    .rokb_battle_data_opponents_skipped_participants = output
                    .metadata
                    .rokb_battle_data_opponents_skipped_participants
                    .saturating_add(skipped_total);
            }
            return Ok(());
        }

        output
            .metadata
            .rokb_battle_data_opponents_skipped_participants = output
            .metadata
            .rokb_battle_data_opponents_skipped_participants
            .saturating_add(skipped_total);
        if let Some(data) = output.battle_data.as_mut() {
            data.opponents = opponents;
        } else {
            output.battle_data = Some(BattleData {
                sender: None,
                opponents,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BattleOpponentsResolver;
    use crate::context::MailContext;
    use crate::resolvers::BattleSenderResolver;
    use crate::structures::BattleMail;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_data(sections: &[Value]) -> BattleMail {
        let ctx = MailContext::new(sections);
        let mut output = BattleMail::default();
        let resolver = BattleOpponentsResolver::new();

        resolver
            .resolve(&ctx, &mut output)
            .expect("resolve battle opponents");

        output
    }

    #[test]
    fn battle_opponents_resolver_reads_opponents_from_ots() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 10,
                            "SideId": 1,
                            "HId": 1
                        }
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "AppUid": "sender",
                "COSId": 100,
                "Abbr": "SND",
                "AName": "SenderAlliance",
                "HId": 1,
                "HLv": 60
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 10,
                        "PName": "Sender",
                        "Abbr": "SND",
                        "HId": 1,
                        "HLv": 60
                    }
                }
            }),
            json!({
                "PId": 99,
                "PName": "Opponent",
                "AppUid": "opp",
                "COSId": 200,
                "CtId": 55,
                "HId": 2,
                "HLv": 40
            }),
            json!({
                "Abbr": "OPP",
                "AName": "OppAlliance"
            }),
            json!({
                "OTs": {
                    "55": {
                        "PId": 99,
                        "PName": "Opponent",
                        "Abbr": "OPP",
                        "HId": 2,
                        "HLv": 40
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");
        assert_eq!(data.opponents.len(), 1);
        let opponent = &data.opponents[0];

        assert_eq!(opponent.player_id, Some(99));
        assert_eq!(opponent.player_name.as_deref(), Some("Opponent"));
        assert_eq!(opponent.app_uid.as_deref(), Some("opp"));
        assert_eq!(opponent.kingdom, Some(200));
        assert_eq!(
            opponent.alliance.as_ref().and_then(|a| a.tag.as_deref()),
            Some("OPP")
        );
        assert_eq!(
            opponent.alliance.as_ref().and_then(|a| a.name.as_deref()),
            Some("OppAlliance")
        );
        assert_eq!(opponent.participants.len(), 1);
        assert_eq!(opponent.participants[0].player_id, Some(99));
    }

    #[test]
    fn battle_opponents_resolver_matches_opponent_ots_by_ctid() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "PId": 21,
                "PName": "OpponentOne",
                "CtId": 101
            }),
            json!({
                "OTs": {
                    "101": {
                        "PId": 21,
                        "PName": "OpponentOne",
                        "Abbr": "OP1"
                    }
                }
            }),
            json!({
                "PId": 22,
                "PName": "OpponentTwo",
                "CtId": 202
            }),
            json!({
                "OTs": {
                    "202": {
                        "PId": 22,
                        "PName": "OpponentTwo",
                        "Abbr": "OP2"
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");

        assert_eq!(data.opponents.len(), 2);
        assert_eq!(data.opponents[0].player_id, Some(21));
        assert_eq!(data.opponents[1].player_id, Some(22));
        assert_eq!(data.opponents[0].participants.len(), 1);
        assert_eq!(data.opponents[1].participants.len(), 1);
    }

    #[test]
    fn battle_opponents_resolver_enriches_from_cidt_with_participant_pid() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "PName": "OpponentMain",
                "AppUid": "opp",
                "COSId": 200,
                "CtId": 0,
                "HId": 2,
                "HLv": 40
            }),
            json!({
                "OTs": {
                    "0": {
                        "PId": 99,
                        "PName": "OpponentMain",
                        "Abbr": "OPP",
                        "HId": 2,
                        "HLv": 40
                    }
                }
            }),
            json!({
                "CIdt": {
                    "PId": 99,
                    "Avatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}",
                    "HId": 2
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");

        assert_eq!(data.opponents.len(), 1);
        assert_eq!(data.opponents[0].player_id, Some(99));
        assert_eq!(data.opponents[0].avatar_url.as_deref(), Some("http://a"));
        assert_eq!(data.opponents[0].frame_url.as_deref(), Some("http://f"));
    }

    #[test]
    fn battle_opponents_resolver_enriches_from_cidt_in_block() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "PName": "OpponentMain",
                "AppUid": "12345",
                "COSId": 200,
                "CtId": 0
            }),
            json!({
                "CIdt": {
                    "PId": 77,
                    "Avatar": "{\"avatar\":\"http://example.com/avatars/12345/img.png\"}",
                    "HId": 10
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");

        assert_eq!(data.opponents.len(), 1);
        assert_eq!(data.opponents[0].player_id, Some(77));
        assert_eq!(
            data.opponents[0]
                .commanders
                .as_ref()
                .and_then(|c| c.primary.as_ref())
                .and_then(|c| c.id),
            Some(10)
        );
    }

    #[test]
    fn battle_opponents_resolver_preserves_sender_data() {
        let sections = vec![
            json!({
                "body": {
                    "content": {
                        "SelfChar": {
                            "PId": 10,
                            "SideId": 1,
                            "HId": 1
                        }
                    }
                }
            }),
            json!({
                "PName": "Sender",
                "COSId": 100,
                "Abbr": "SND",
                "HId": 1,
                "HLv": 60
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 10,
                        "PName": "Sender",
                        "Abbr": "SND",
                        "HId": 1,
                        "HLv": 60
                    }
                }
            }),
            json!({
                "PName": "Opponent",
                "CtId": 55,
                "HId": 2,
                "HLv": 40
            }),
            json!({
                "OTs": {
                    "55": {
                        "PId": 99,
                        "PName": "Opponent",
                        "Abbr": "OPP",
                        "HId": 2,
                        "HLv": 40
                    }
                }
            }),
        ];

        let ctx = MailContext::new(&sections);
        let mut output = BattleMail::default();
        let sender_resolver = BattleSenderResolver::new();
        let opponents_resolver = BattleOpponentsResolver::new();

        sender_resolver
            .resolve(&ctx, &mut output)
            .expect("resolve sender");
        opponents_resolver
            .resolve(&ctx, &mut output)
            .expect("resolve opponents");

        let data = output.battle_data.expect("battle data");
        assert!(data.sender.is_some());
        assert_eq!(data.opponents.len(), 1);
    }

    #[test]
    fn battle_opponents_resolver_tracks_skipped_participants() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "PName": "Opponent",
                "CtId": 101
            }),
            json!({
                "OTs": {
                    "-2": {
                        "PId": 1,
                        "PName": "",
                        "HId": 0,
                        "HLv": 0
                    },
                    "101": {
                        "PId": 21,
                        "PName": "Opponent",
                        "Abbr": "OP1"
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        assert_eq!(
            output
                .metadata
                .rokb_battle_data_opponents_skipped_participants,
            1
        );
    }

    #[test]
    fn battle_opponents_resolver_falls_back_to_ots_only() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "OTs": {
                    "101": {
                        "PId": 21,
                        "PName": "OpponentOne",
                        "Abbr": "OP1",
                        "HId": 10,
                        "HLv": 60
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");

        assert_eq!(data.opponents.len(), 1);
        assert!(data.opponents[0].player_id.is_none());
        assert!(data.opponents[0].player_name.is_none());
        assert_eq!(data.opponents[0].participants.len(), 1);
        assert_eq!(
            data.opponents[0].participants[0].player_name.as_deref(),
            Some("OpponentOne")
        );
    }

    #[test]
    fn battle_opponents_resolver_uses_cidt_in_ots_only_sections() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "Attacks": {
                    "1": {
                        "CIdt": {
                            "PId": 44,
                            "SideId": 7
                        }
                    },
                    "OTs": {
                        "101": {
                            "PName": "OpponentOne",
                            "Abbr": "OP1"
                        }
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");

        assert_eq!(data.opponents.len(), 1);
        assert_eq!(data.opponents[0].camp, Some(7));
    }

    #[test]
    fn battle_opponents_resolver_uses_attack_seed_when_present() {
        let sections = vec![
            json!({
                "PName": "Sender",
                "AppUid": "sender"
            }),
            json!({
                "STs": {
                    "0": {
                        "PId": 1,
                        "PName": "Sender",
                        "Abbr": "SND"
                    }
                }
            }),
            json!({
                "Attacks": {
                    "PName": "OpponentMain",
                    "AppUid": "opp",
                    "HId": 10,
                    "HLv": 50,
                    "HId2": 24,
                    "HLv2": 1,
                    "1": {
                        "CIdt": {
                            "PId": 77,
                            "Avatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}",
                            "HId": 11,
                            "HLv": 60,
                            "HAw2": false
                        }
                    }
                },
                "OTs": {
                    "101": {
                        "PId": 21,
                        "PName": "OpponentParticipant",
                        "Abbr": "OP1"
                    }
                }
            }),
        ];

        let output = resolve_data(&sections);
        let data = output.battle_data.expect("battle data");

        assert_eq!(data.opponents.len(), 1);
        assert_eq!(
            data.opponents[0].player_name.as_deref(),
            Some("OpponentMain")
        );
        assert_eq!(data.opponents[0].player_id, Some(77));
        assert_eq!(data.opponents[0].avatar_url.as_deref(), Some("http://a"));
        assert_eq!(data.opponents[0].frame_url.as_deref(), Some("http://f"));
        assert_eq!(
            data.opponents[0]
                .commanders
                .as_ref()
                .and_then(|c| c.secondary.as_ref())
                .and_then(|c| c.awakened),
            Some(false)
        );
        assert_eq!(data.opponents[0].participants.len(), 1);
        assert_eq!(
            data.opponents[0].participants[0].player_name.as_deref(),
            Some("OpponentParticipant")
        );
    }
}
