use crate::{
    helpers::*,
    resolvers::{Resolver, ResolverContext},
};
use serde_json::Value;

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
}

impl Resolver for BattleResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let group = ctx.group;
        let (_idx, atk_block) = find_best_attack_block(group, ctx.attack_id);
        let damage = atk_block.get("Damage").cloned().unwrap_or(Value::Null);
        let kill = atk_block.get("Kill").cloned().unwrap_or(Value::Null);

        let results_obj = get_or_insert_object(mail, "battle_results");
        if let Some(obj) = results_obj.as_object_mut() {
            // self
            if let Some(v) = get_i64_alt(&damage, "Power", "AtkPower") {
                obj.insert("power".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("InitMax").and_then(|x| x.as_i64()) {
                obj.insert("init_max".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("Max").and_then(|x| x.as_i64()) {
                obj.insert("max".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("Healing").and_then(|x| x.as_i64()) {
                obj.insert("healing".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("Death").and_then(|x| x.as_i64()) {
                obj.insert("death".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("BadHurt").and_then(|x| x.as_i64()) {
                obj.insert("severely_wounded".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("Hurt").and_then(|x| x.as_i64()) {
                obj.insert("wounded".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("Cnt").and_then(|x| x.as_i64()) {
                obj.insert("remaining".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("Gt").and_then(|x| x.as_i64()) {
                obj.insert("watchtower".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("GtMax").and_then(|x| x.as_i64()) {
                obj.insert("watchtower_max".to_string(), Value::from(v));
            }
            if let Some(v) = damage.get("KillScore").and_then(|x| x.as_i64()) {
                obj.insert("kill_score".to_string(), Value::from(v));
            }

            // enemy
            if let Some(v) = get_i64_alt(&kill, "Power", "AtkPower") {
                obj.insert("enemy_power".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("InitMax").and_then(|x| x.as_i64()) {
                obj.insert("enemy_init_max".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("Max").and_then(|x| x.as_i64()) {
                obj.insert("enemy_max".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("Healing").and_then(|x| x.as_i64()) {
                obj.insert("enemy_healing".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("Death").and_then(|x| x.as_i64()) {
                obj.insert("enemy_death".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("BadHurt").and_then(|x| x.as_i64()) {
                obj.insert("enemy_severely_wounded".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("Hurt").and_then(|x| x.as_i64()) {
                obj.insert("enemy_wounded".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("Cnt").and_then(|x| x.as_i64()) {
                obj.insert("enemy_remaining".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("Gt").and_then(|x| x.as_i64()) {
                obj.insert("enemy_watchtower".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("GtMax").and_then(|x| x.as_i64()) {
                obj.insert("enemy_watchtower_max".to_string(), Value::from(v));
            }
            if let Some(v) = kill.get("KillScore").and_then(|x| x.as_i64()) {
                obj.insert("enemy_kill_score".to_string(), Value::from(v));
            }
        }

        Ok(())
    }
}
