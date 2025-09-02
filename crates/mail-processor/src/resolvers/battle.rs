use crate::{
    helpers::{find_best_attack_block_ref, get_or_insert_object},
    resolvers::{Resolver, ResolverContext},
};
use serde_json::{Map, Value};

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

    fn json_as_obj(v: Option<&Value>) -> Option<&Map<String, Value>> {
        v.and_then(Value::as_object)
    }

    fn i64_field(m: &Map<String, Value>, key: &str) -> Option<i64> {
        m.get(key).and_then(Value::as_i64)
    }

    fn i64_any(m: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
        keys.iter().find_map(|k| Self::i64_field(m, k))
    }

    fn put_i64_pref(dst: &mut Map<String, Value>, prefix: &str, name: &str, val: Option<i64>) {
        if let Some(v) = val {
            dst.insert(format!("{prefix}{name}"), Value::from(v));
        }
    }

    fn copy_std_fields(dst: &mut Map<String, Value>, src: &Map<String, Value>, prefix: &str) {
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
            Self::put_i64_pref(dst, prefix, to, Self::i64_field(src, from));
        }
    }

    fn copy_side(dst: &mut Map<String, Value>, src: Option<&Map<String, Value>>, prefix: &str) {
        if let Some(m) = src {
            Self::put_i64_pref(
                dst,
                prefix,
                "power",
                Self::i64_any(m, &["Power", "AtkPower"]),
            );
            Self::copy_std_fields(dst, m, prefix);
        }
    }

    fn find_nearby_obj<'a>(
        group: &'a [Value],
        start: usize,
        key: &str,
        max_span: usize,
    ) -> Option<&'a Map<String, Value>> {
        for d in 1..=max_span {
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
}

impl Resolver for BattleResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let group = ctx.group;
        let (idx_opt, atk_block_opt) = find_best_attack_block_ref(group, ctx.attack_id);

        let section = idx_opt.and_then(|i| group.get(i)).unwrap_or(&Value::Null);
        let atk_block = atk_block_opt.unwrap_or(section);

        let damage = Self::json_as_obj(atk_block.get("Damage"))
            .or_else(|| Self::json_as_obj(section.get("Damage")))
            .or_else(|| idx_opt.and_then(|i| Self::find_nearby_obj(group, i, "Damage", 3)));

        let kill = Self::json_as_obj(atk_block.get("Kill"))
            .or_else(|| Self::json_as_obj(section.get("Kill")))
            .or_else(|| idx_opt.and_then(|i| Self::find_nearby_obj(group, i, "Kill", 3)));

        let obj = match get_or_insert_object(mail, "battle_results") {
            Value::Object(m) => m,
            _ => unreachable!("battle_results must be an object"),
        };

        // self
        Self::copy_side(obj, damage, "");
        // enemy
        Self::copy_side(obj, kill, "enemy_");

        Ok(())
    }
}
