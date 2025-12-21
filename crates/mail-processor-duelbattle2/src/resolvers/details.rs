use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::Value;

use crate::context::MailContext;
use crate::helpers::{
    build_commander, collect_buffs, find_detail_context, find_first_section_with_key,
    is_skill_only_object, parse_avatar, parse_bool, parse_i64, parse_string, push_skill_from_value,
    split_sections,
};
use crate::structures::{
    DuelBattle2Commander, DuelBattle2Mail, DuelBattle2Participant, DuelBattle2Results,
};

/// Resolves sender, opponent, and results data from DuelBattle2 sections.
pub struct DetailsResolver;

impl Default for DetailsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DetailsResolver {
    pub fn new() -> Self {
        Self
    }

    fn fill_participant(
        participant: &mut DuelBattle2Participant,
        player: &Value,
        parent: &Value,
        sections: &[Value],
    ) {
        if participant.player_name.is_none() {
            participant.player_name = player.get("PlayerName").and_then(parse_string);
        }
        if participant.kingdom.is_none() {
            participant.kingdom = player.get("ServerId").and_then(parse_i64);
        }
        if participant.alliance.is_none() {
            participant.alliance = player.get("Abbr").and_then(parse_string);
        }

        if let Some(identity) = find_first_section_with_key(sections, "PlayerId") {
            if participant.player_id.is_none() {
                participant.player_id = identity.get("PlayerId").and_then(parse_i64);
            }
            if participant.duel_id.is_none() {
                participant.duel_id = identity.get("DuelTeamId").and_then(parse_i64);
            }
            if let Some(avatar_value) = identity.get("PlayerAvatar") {
                let (avatar, frame) = parse_avatar(avatar_value);
                if participant.avatar_url.is_none() {
                    participant.avatar_url = avatar;
                }
                if participant.frame_url.is_none() {
                    participant.frame_url = frame;
                }
            }
        }

        if participant.buffs.is_empty() {
            participant.buffs = collect_buffs(sections);
        }

        if participant.commanders.primary.is_none() {
            participant.commanders.primary = Self::build_primary_commander(player, parent);
        }
        if participant.commanders.secondary.is_none() {
            participant.commanders.secondary = Self::build_secondary_commander(sections);
        }
    }

    fn fill_results(
        results: &mut DuelBattle2Results,
        player: &Value,
        sections: &[Value],
        is_opponent: bool,
    ) {
        let identity = find_first_section_with_key(sections, "PlayerId");

        let kill_points = player.get("KillScore").and_then(parse_i64);
        let sev_wounded = player.get("UnitBadHurt").and_then(parse_i64);
        let wounded = player.get("UnitHurt").and_then(parse_i64);
        let dead = player.get("UnitDead").and_then(parse_i64);
        let heal = player.get("UnitReturn").and_then(parse_i64);
        let win = player.get("IsWin").and_then(parse_bool);

        let units = identity.and_then(|s| s.get("UnitTotal").and_then(parse_i64));
        let power = identity.and_then(|s| s.get("LosePower").and_then(parse_i64));

        if is_opponent {
            results.opponent_kill_points = kill_points;
            results.opponent_sev_wounded = sev_wounded;
            results.opponent_wounded = wounded;
            results.opponent_dead = dead;
            results.opponent_heal = heal;
            results.opponent_units = units;
            results.opponent_power = power;
            results.opponent_win = win;
        } else {
            results.kill_points = kill_points;
            results.sev_wounded = sev_wounded;
            results.wounded = wounded;
            results.dead = dead;
            results.heal = heal;
            results.units = units;
            results.power = power;
            results.win = win;
        }
    }

    fn build_primary_commander(player: &Value, parent: &Value) -> Option<DuelBattle2Commander> {
        let heroes = player.get("Heroes")?;
        let main_hero = heroes.get("MainHero")?;

        let mut primary = build_commander(main_hero);
        let mut skills = Vec::new();

        if let Some(skills_value) = main_hero.get("Skills") {
            push_skill_from_value(skills_value, &mut skills);
        }

        // Skills can bleed out from nested hero objects to their parents.
        push_skill_from_value(main_hero, &mut skills);
        push_skill_from_value(heroes, &mut skills);
        push_skill_from_value(player, &mut skills);
        push_skill_from_value(parent, &mut skills);

        primary.skills = skills;

        Some(primary)
    }

    fn build_secondary_commander(sections: &[Value]) -> Option<DuelBattle2Commander> {
        let (assist_index, assist_section, assist) = sections
            .iter()
            .enumerate()
            .find_map(|(idx, section)| section.get("AssistHero").map(|v| (idx, section, v)))?;

        let mut secondary = build_commander(assist);
        let mut skills = Vec::new();

        if let Some(skills_value) = assist.get("Skills") {
            push_skill_from_value(skills_value, &mut skills);
        }

        push_skill_from_value(assist, &mut skills);
        // Include skills that bleed into the parent assist hero section.
        push_skill_from_value(assist_section, &mut skills);

        // Collect skill-only bleed-out sections that follow the assist hero section.
        for section in sections.iter().skip(assist_index + 1) {
            let Some(obj) = section.as_object() else {
                break;
            };
            if !is_skill_only_object(obj) {
                break;
            }
            push_skill_from_value(section, &mut skills);
        }

        secondary.skills = skills;

        Some(secondary)
    }
}

impl Resolver<MailContext<'_>, DuelBattle2Mail> for DetailsResolver {
    type Error = Infallible;

    fn resolve(
        &self,
        ctx: &MailContext<'_>,
        output: &mut DuelBattle2Mail,
    ) -> Result<(), Self::Error> {
        let sections = ctx.sections;
        let detail = find_detail_context(sections);
        // Split sections at the defender marker to keep bleed-out data aligned per side.
        let (attacker_sections, defender_sections) = split_sections(sections, detail.def_index());

        if let Some(attacker) = detail.attacker {
            Self::fill_participant(
                &mut output.sender,
                attacker.player,
                attacker.parent,
                attacker_sections,
            );
            Self::fill_results(
                &mut output.results,
                attacker.player,
                attacker_sections,
                false,
            );
        }

        if let Some(defender) = detail.defender {
            Self::fill_participant(
                &mut output.opponent,
                defender.player,
                defender.parent,
                defender_sections,
            );
            Self::fill_results(
                &mut output.results,
                defender.player,
                defender_sections,
                true,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DetailsResolver;
    use crate::context::MailContext;
    use crate::structures::DuelBattle2Mail;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_details(sections: Vec<Value>) -> DuelBattle2Mail {
        let ctx = MailContext::new(&sections);
        let mut output = DuelBattle2Mail::default();
        let resolver = DetailsResolver::new();

        resolver
            .resolve(&ctx, &mut output)
            .expect("resolve details");

        output
    }

    #[test]
    fn details_resolver_extracts_details_and_results() {
        let sections = vec![
            json!({
                "id": "mail-3",
                "time": 999,
                "serverId": 15790,
                "body": {
                    "detail": {
                        "AtkPlayer": {
                            "PlayerName": "Attacker",
                            "ServerId": 1804,
                            "Abbr": "AAA",
                            "KillScore": 10,
                            "UnitBadHurt": 2,
                            "UnitHurt": 3,
                            "UnitDead": 4,
                            "UnitReturn": 5,
                            "IsWin": false,
                            "Heroes": {
                                "MainHero": {
                                    "HeroId": 100,
                                    "HeroLevel": 60,
                                    "Star": 6,
                                    "Awaked": true,
                                    "Skills": { "Id": 0, "Level": 5, "SkillId": 1000 },
                                    "Id": 1,
                                    "Level": 2,
                                    "SkillId": 1001
                                },
                                "Id": 2,
                                "Level": 3,
                                "SkillId": 1002
                            },
                            "Id": 3,
                            "Level": 4,
                            "SkillId": 1003
                        },
                        "Id": 4,
                        "Level": 1,
                        "SkillId": 1004
                    }
                }
            }),
            json!({ "Buffs": { "BuffId": 11, "BuffValue": 1 }, "BuffId": 12, "BuffValue": 2 }),
            json!({
                "AssistHero": {
                    "HeroId": 101,
                    "HeroLevel": 50,
                    "Star": 5,
                    "Awaked": false,
                    "Skills": { "Id": 0, "Level": 1, "SkillId": 2000 },
                    "Id": 1,
                    "Level": 2,
                    "SkillId": 2001
                },
                "Id": 2,
                "Level": 3,
                "SkillId": 2002
            }),
            json!({ "Id": 3, "Level": 4, "SkillId": 2003 }),
            json!({ "PosIdx": 11 }),
            json!({
                "DuelTeamId": 111,
                "UnitTotal": 999,
                "PlayerId": 42,
                "LosePower": 888,
                "PlayerAvatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}"
            }),
            json!({
                "DefPlayer": {
                    "PlayerName": "Defender",
                    "ServerId": 1900,
                    "Abbr": "BBB",
                    "KillScore": 20,
                    "UnitBadHurt": 12,
                    "UnitHurt": 13,
                    "UnitDead": 14,
                    "UnitReturn": 15,
                    "IsWin": true,
                    "Heroes": {
                        "MainHero": {
                            "HeroId": 200,
                            "HeroLevel": 40,
                            "Star": 4,
                            "Awaked": false,
                            "Skills": { "Id": 0, "Level": 1, "SkillId": 3000 },
                            "Id": 1,
                            "Level": 2,
                            "SkillId": 3001
                        }
                    },
                    "Id": 2,
                    "Level": 3,
                    "SkillId": 3002
                },
                "Id": 3,
                "Level": 4,
                "SkillId": 3003
            }),
            json!({ "BuffId": 21, "BuffValue": 3.5 }),
            json!({
                "AssistHero": {
                    "HeroId": 201,
                    "HeroLevel": 41,
                    "Star": 4,
                    "Awaked": true,
                    "Skills": { "Id": 0, "Level": 1, "SkillId": 4000 },
                    "Id": 1,
                    "Level": 2,
                    "SkillId": 4001
                },
                "Id": 2,
                "Level": 3,
                "SkillId": 4002
            }),
            json!({ "Id": 3, "Level": 4, "SkillId": 4003 }),
            json!({
                "DuelTeamId": 222,
                "UnitTotal": 555,
                "PlayerId": 84,
                "LosePower": 444,
                "PlayerAvatar": "{\"avatar\":\"http://b\",\"avatarFrame\":\"http://g\"}"
            }),
        ];

        let output = resolve_details(sections);

        assert_eq!(output.sender.player_name.as_deref(), Some("Attacker"));
        assert_eq!(output.sender.player_id, Some(42));
        assert_eq!(output.sender.duel_id, Some(111));
        assert_eq!(output.sender.avatar_url.as_deref(), Some("http://a"));
        assert_eq!(output.sender.frame_url.as_deref(), Some("http://f"));
        assert_eq!(output.sender.buffs.len(), 2);

        let primary = output
            .sender
            .commanders
            .primary
            .as_ref()
            .expect("primary commander");
        assert_eq!(primary.id, Some(100));
        assert_eq!(primary.skills.len(), 5);

        let secondary = output
            .sender
            .commanders
            .secondary
            .as_ref()
            .expect("secondary commander");
        assert_eq!(secondary.id, Some(101));
        assert_eq!(secondary.skills.len(), 4);

        assert_eq!(output.results.kill_points, Some(10));
        assert_eq!(output.results.sev_wounded, Some(2));
        assert_eq!(output.results.wounded, Some(3));
        assert_eq!(output.results.dead, Some(4));
        assert_eq!(output.results.heal, Some(5));
        assert_eq!(output.results.units, Some(999));
        assert_eq!(output.results.power, Some(888));
        assert_eq!(output.results.win, Some(false));

        assert_eq!(output.opponent.player_name.as_deref(), Some("Defender"));
        assert_eq!(output.opponent.player_id, Some(84));
        assert_eq!(output.opponent.duel_id, Some(222));

        let opponent_primary = output
            .opponent
            .commanders
            .primary
            .as_ref()
            .expect("opponent primary commander");
        assert_eq!(opponent_primary.id, Some(200));
        assert_eq!(opponent_primary.skills.len(), 4);

        let opponent_secondary = output
            .opponent
            .commanders
            .secondary
            .as_ref()
            .expect("opponent secondary commander");
        assert_eq!(opponent_secondary.id, Some(201));
        assert_eq!(opponent_secondary.skills.len(), 4);

        assert_eq!(output.results.opponent_kill_points, Some(20));
        assert_eq!(output.results.opponent_sev_wounded, Some(12));
        assert_eq!(output.results.opponent_wounded, Some(13));
        assert_eq!(output.results.opponent_dead, Some(14));
        assert_eq!(output.results.opponent_heal, Some(15));
        assert_eq!(output.results.opponent_units, Some(555));
        assert_eq!(output.results.opponent_power, Some(444));
        assert_eq!(output.results.opponent_win, Some(true));
    }
}
