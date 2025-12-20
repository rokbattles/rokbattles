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
