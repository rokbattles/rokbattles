pub mod helpers;
pub mod resolvers;
pub mod structures;

use crate::{
    resolvers::ResolverChain,
    resolvers::ResolverContext,
    resolvers::battle::BattleResolver,
    resolvers::metadata::MetadataResolver,
    resolvers::participant_enemy::ParticipantEnemyResolver,
    resolvers::participant_self::ParticipantSelfResolver,
    structures::{DecodedMail, ParsedMail},
};
use anyhow::{Result, bail};
use serde_json::json;
use std::cmp::Ordering;

pub fn process(json_text: &str) -> Result<Vec<ParsedMail>> {
    let root: DecodedMail = serde_json::from_str(json_text)?;
    let sections = &root.sections;

    if sections.is_empty() {
        return Ok(Vec::new());
    }

    struct Battle {
        index: usize,
        attack_id: String,
    }

    let mut battles: Vec<Battle> = Vec::new();

    for (index, section) in sections.iter().enumerate() {
        if let Some(obj) = section.as_object() {
            // The first attack is at the top level section
            for key in obj.keys() {
                if key.chars().all(|c| c.is_ascii_digit()) {
                    battles.push(Battle {
                        index,
                        attack_id: key.to_string(),
                    })
                }
            }

            // All other attacks are in an Attacks object
            if let Some(att) = obj.get("Attacks").and_then(|v| v.as_object()) {
                for key in att.keys() {
                    if key.chars().all(|c| c.is_ascii_digit()) {
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
        bail!("No battles found in mail")
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
        .with(BattleResolver::new());

    let mut entries: Vec<ParsedMail> = Vec::new();

    for (attack_id, start, end) in battle_groups {
        let mut entry = json!({
            "metadata": {},
            "self": {},
            "enemy": {},
            "battle_results": {}
        });

        let ctx = ResolverContext {
            sections,
            group: &sections[start..end],
            attack_id: &attack_id,
        };

        chain.apply(&ctx, &mut entry)?;

        let parsed: ParsedMail = serde_json::from_value(entry)?;
        entries.push(parsed);
    }

    Ok(entries)
}
