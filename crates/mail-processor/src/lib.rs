pub mod helpers;
pub mod resolvers;
pub mod structures;

use crate::{
    resolvers::ResolverChain,
    resolvers::ResolverContext,
    resolvers::battle::BattleResolver,
    resolvers::metadata::MetadataResolver,
    resolvers::participant::ParticipantResolver,
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

    // [start, end) groups per attack id
    let mut battle_groups = Vec::new();
    for pair in battles.windows(2) {
        battle_groups.push((pair[0].attack_id.clone(), pair[0].index, pair[1].index));
    }

    let last = battles.last().unwrap();
    battle_groups.push((last.attack_id.clone(), last.index, sections.len()));

    let chain = ResolverChain::new()
        .with(MetadataResolver::new())
        .with(ParticipantResolver::new())
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
