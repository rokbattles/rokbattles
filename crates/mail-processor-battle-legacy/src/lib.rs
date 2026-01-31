pub mod context;
pub mod error;
pub mod helpers;
pub mod resolvers;
pub mod structures;

use crate::{
    context::MailContext, error::ProcessError, helpers::is_ascii_digits,
    resolvers::battle::BattleResolver, resolvers::metadata::MetadataResolver,
    resolvers::overview::OverviewResolver, resolvers::participant_enemy::ParticipantEnemyResolver,
    resolvers::participant_self::ParticipantSelfResolver, structures::ParsedMail,
};
use mail_processor_sdk_legacy::ResolverChain;
use serde_json::{Value, json};
use std::cmp::Ordering;

pub fn process_sections(sections: &[Value]) -> Result<Vec<ParsedMail>, ProcessError> {
    if sections.is_empty() {
        return Ok(Vec::new());
    }

    struct Battle {
        index: usize,
        attack_id: String,
    }

    let mut battles: Vec<Battle> = Vec::new();

    // TODO this is the incorrect way of handling attacks. we'll fix it at a later date since this
    //  produces the same output, but very inefficiently.
    for (index, section) in sections.iter().enumerate() {
        if let Some(obj) = section.as_object() {
            if let Some(att) = obj.get("Attacks").and_then(|v| v.as_object()) {
                for key in att.keys() {
                    if is_ascii_digits(key) {
                        // Flat "Attacks" map where each numeric key represents a battle.
                        battles.push(Battle {
                            index,
                            attack_id: key.to_string(),
                        })
                    }
                }
            }

            for (key, val) in obj.iter() {
                if is_ascii_digits(key)
                    && let Some(m) = val.as_object()
                {
                    // Some battle payloads are stored directly on the section with the attack id as the key.
                    let has_battle = m.get("Kill").is_some()
                        || m.get("Damage").is_some()
                        || m.get("CIdt").is_some();
                    if has_battle {
                        battles.push(Battle {
                            index,
                            attack_id: key.to_string(),
                        })
                    }
                }
            }
        }
    }

    if battles.is_empty() {
        return Err(ProcessError::NoBattles);
    }

    // Order and deduplicate
    battles.sort_by(|a, b| match a.index.cmp(&b.index) {
        Ordering::Equal => a.attack_id.cmp(&b.attack_id),
        ord => ord,
    });
    battles.dedup_by(|a, b| a.index == b.index && a.attack_id == b.attack_id);

    let mut battle_groups = Vec::new();

    let mut boundaries: Vec<usize> = battles.iter().map(|b| b.index).collect();
    boundaries.sort_unstable();
    boundaries.dedup();

    for b in &battles {
        // Group spans from the current battle section to (but not including) the next battle section.
        let end = boundaries
            .iter()
            .copied()
            .find(|&i| i > b.index)
            .unwrap_or(sections.len());
        battle_groups.push((b.attack_id.clone(), b.index, end));
    }

    let chain = ResolverChain::new()
        .with(MetadataResolver::new())
        .with(ParticipantSelfResolver::new())
        .with(ParticipantEnemyResolver::new())
        .with(OverviewResolver::new())
        .with(BattleResolver::new());

    let mut entries: Vec<ParsedMail> = Vec::new();

    for (attack_id, start, end) in &battle_groups {
        let mut entry = json!({
            "metadata": {},
            "self": {},
            "enemy": {},
            "overview": {},
            "battle_results": {}
        });

        let ctx = MailContext::new(sections, &sections[*start..*end], attack_id.as_str());

        chain.apply(&ctx, &mut entry)?;

        let keep = entry
            .pointer("/battle_results")
            .and_then(|v| v.as_object())
            .map(|m| m.values().any(|v| !v.is_null()))
            .unwrap_or(false);
        if !keep {
            continue;
        }

        let parsed: ParsedMail = serde_json::from_value(entry)?;
        entries.push(parsed);
    }

    Ok(entries)
}
