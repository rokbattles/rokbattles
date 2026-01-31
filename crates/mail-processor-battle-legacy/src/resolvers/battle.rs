use crate::{
    context::MailContext,
    helpers::{
        find_attack_block_best_match, get_or_insert_object_map, map_get_i64,
        map_put_i64_with_prefix, value_matches_attack_id,
    },
};
use mail_processor_sdk_legacy::Resolver;
use serde_json::{Map, Value};
use std::convert::Infallible;

pub struct BattleResolver;

impl Default for BattleResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleResolver {
    pub fn new() -> Self {
        Self
    }

    fn as_object_opt(v: Option<&Value>) -> Option<&Map<String, Value>> {
        v.and_then(Value::as_object)
    }

    fn copy_standard_stat_fields(
        dst: &mut Map<String, Value>,
        src: &Map<String, Value>,
        prefix: &str,
    ) {
        const FIELDS: &[(&str, &str)] = &[
            ("InitMax", "init_max"),
            ("Max", "max"),
            ("Healing", "healing"),
            ("Death", "death"),
            ("BadHurt", "severely_wounded"),
            ("Hurt", "wounded"),
            ("Cnt", "remaining"),
            ("Gt", "watchtower"),
            ("GtMax", "watchtower_max"),
            ("KillScore", "kill_score"),
        ];
        for &(from, to) in FIELDS {
            map_put_i64_with_prefix(dst, prefix, to, map_get_i64(src, from));
        }
    }

    fn copy_side_stats(
        dst: &mut Map<String, Value>,
        src: Option<&Map<String, Value>>,
        prefix: &str,
    ) {
        if let Some(m) = src {
            map_put_i64_with_prefix(dst, prefix, "power", map_get_i64(m, "Power"));
            map_put_i64_with_prefix(dst, prefix, "attack_power", map_get_i64(m, "AtkPower"));
            map_put_i64_with_prefix(dst, prefix, "skill_power", map_get_i64(m, "SkillPower"));
            map_put_i64_with_prefix(dst, prefix, "reinforcements_join", map_get_i64(m, "AddCnt"));
            map_put_i64_with_prefix(
                dst,
                prefix,
                "reinforcements_retreat",
                map_get_i64(m, "RetreatCnt"),
            );
            map_put_i64_with_prefix(dst, prefix, "acclaim", map_get_i64(m, "Contribute"));
            Self::copy_standard_stat_fields(dst, m, prefix);
        }
    }

    fn find_nearby_object_by_key<'a>(
        group: &'a [Value],
        start: usize,
        key: &str,
        max_span: usize,
    ) -> Option<&'a Map<String, Value>> {
        for d in 1..=max_span {
            // Walk outward symmetrically to keep the closest match for this key.
            if start >= d
                && let Some(m) = group
                    .get(start - d)
                    .and_then(|s| s.get(key))
                    .and_then(Value::as_object)
            {
                return Some(m);
            }
            if let Some(m) = group
                .get(start + d)
                .and_then(|s| s.get(key))
                .and_then(Value::as_object)
            {
                return Some(m);
            }
        }
        None
    }

    fn find_closest_object_by_key<'a>(
        group: &'a [Value],
        start: Option<usize>,
        key: &str,
    ) -> Option<&'a Map<String, Value>> {
        let anchor = start.unwrap_or(0);
        let mut best: Option<(usize, &Map<String, Value>)> = None;
        for (i, s) in group.iter().enumerate() {
            if let Some(m) = s.get(key).and_then(Value::as_object) {
                let dist = anchor.abs_diff(i);
                // Keep the closest object to the anchor index.
                if best.map(|(d, _)| dist < d).unwrap_or(true) {
                    best = Some((dist, m));
                }
            }
        }
        best.map(|(_, m)| m)
    }

    fn match_idt_in_sections(sections: &[Value], attack_id: &str) -> Option<usize> {
        for (i, s) in sections.iter().enumerate() {
            let idt_match = s
                .get("Idt")
                .map(|v| value_matches_attack_id(v, attack_id))
                .unwrap_or(false);
            if idt_match
                && (s.get("HSS").is_some() || s.get("HId").is_some() || s.get("HId2").is_some())
            {
                return Some(i);
            }
        }
        None
    }

    fn find_attacks_object_for_id<'a>(
        sections: &'a [Value],
        attack_id: &str,
    ) -> Option<&'a Map<String, Value>> {
        for s in sections {
            if let Some(attacks) = s.get("Attacks").and_then(Value::as_object)
                && let Some(b) = attacks.get(attack_id).and_then(Value::as_object)
            {
                // Prefer nested Attacks.{id} blocks when available.
                return Some(b);
            }
            if let Some(b) = s.get(attack_id).and_then(Value::as_object)
                && (b.get("Kill").is_some() || b.get("Damage").is_some() || b.get("CIdt").is_some())
            {
                return Some(b);
            }
        }
        None
    }
}

impl Resolver<MailContext<'_>, Value> for BattleResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, mail: &mut Value) -> Result<(), Self::Error> {
        let group = ctx.group;
        let (idx_opt, atk_block_opt) = find_attack_block_best_match(group, ctx.attack_id);
        let attack_idx = group.iter().position(|s| s.get(ctx.attack_id).is_some());
        // Anchor around whichever section most confidently matches this attack id.
        let anchor_in_group = attack_idx.or(idx_opt);

        let section = idx_opt.and_then(|i| group.get(i)).unwrap_or(&Value::Null);
        let atk_block = atk_block_opt.unwrap_or(section);

        let damage = Self::as_object_opt(atk_block.get("Damage"))
            .or_else(|| Self::as_object_opt(section.get("Damage")))
            .or_else(|| {
                anchor_in_group.and_then(|i| Self::find_nearby_object_by_key(group, i, "Damage", 8))
            });

        let kill = Self::as_object_opt(atk_block.get("Kill"))
            .or_else(|| Self::as_object_opt(section.get("Kill")))
            .or_else(|| {
                anchor_in_group.and_then(|i| Self::find_nearby_object_by_key(group, i, "Kill", 8))
            });

        let obj = get_or_insert_object_map(mail, "battle_results");

        let mut damage_opt = damage;
        let mut kill_opt = kill;

        if damage_opt.is_none() || kill_opt.is_none() {
            let full_sections = ctx.sections;
            let anchor_idx = Self::match_idt_in_sections(full_sections, ctx.attack_id).or(idx_opt);

            let attacks_obj = Self::find_attacks_object_for_id(full_sections, ctx.attack_id);

            if damage_opt.is_none() {
                // Fall back to other sections in the mail that reference this attack id.
                damage_opt = attacks_obj
                    .and_then(|m| m.get("Damage").and_then(Value::as_object))
                    .or_else(|| {
                        let key = ctx.attack_id;
                        full_sections.iter().find_map(|s| {
                            s.get("Attacks")
                                .and_then(Value::as_object)
                                .and_then(|a| a.get(key))
                                .and_then(|_| {
                                    s.get("Attacks")
                                        .and_then(Value::as_object)
                                        .and_then(|a| a.get("Damage"))
                                        .and_then(Value::as_object)
                                })
                        })
                    })
                    .or_else(|| {
                        anchor_idx.and_then(|i| {
                            Self::find_nearby_object_by_key(full_sections, i, "Damage", 16)
                        })
                    })
                    .or_else(|| {
                        full_sections
                            .iter()
                            .find(|s| {
                                s.get("Attacks")
                                    .and_then(Value::as_object)
                                    .is_some_and(|a| a.get(ctx.attack_id).is_some())
                            })
                            .and_then(|s| s.get("Damage").and_then(Value::as_object))
                    })
                    .or_else(|| {
                        anchor_idx
                            .and_then(|i| full_sections.get(i))
                            .and_then(|s| s.get("Damage").and_then(Value::as_object))
                    });
            }

            if kill_opt.is_none() {
                // Kill stats sometimes live near the attack meta, not inside the primary block.
                kill_opt = attacks_obj
                    .and_then(|m| m.get("Kill").and_then(Value::as_object))
                    .or_else(|| {
                        anchor_idx.and_then(|i| {
                            Self::find_nearby_object_by_key(full_sections, i, "Kill", 16)
                        })
                    });
            }
        }

        if damage_opt.is_none() {
            damage_opt = Self::find_closest_object_by_key(group, anchor_in_group, "Damage");
        }
        if kill_opt.is_none() {
            kill_opt = Self::find_closest_object_by_key(group, anchor_in_group, "Kill");
        }

        Self::copy_side_stats(obj, damage_opt, "");
        Self::copy_side_stats(obj, kill_opt, "enemy_");

        Ok(())
    }
}
