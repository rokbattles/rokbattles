use crate::{
    helpers::*,
    resolvers::{Resolver, ResolverContext},
};
use serde_json::Value;

pub struct MetadataResolver;

impl Default for MetadataResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for MetadataResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let metadata = get_or_insert_object(mail, "metadata");
        let sections = ctx.sections;
        let group = ctx.group;

        if let Some(meta) = metadata.as_object_mut() {
            // attack id
            meta.entry("attack_id")
                .or_insert_with(|| Value::String(ctx.attack_id.to_string()));

            // email basics
            let g0 = sections.first().cloned().unwrap_or(Value::Null);
            if let Some(id) = g0.get("id").and_then(|v| v.as_str()) {
                meta.entry("email_id")
                    .or_insert(Value::String(id.to_string()));
            }
            if let Some(tp) = g0.get("type").and_then(|v| v.as_str()) {
                meta.entry("email_type")
                    .or_insert(Value::String(tp.to_string()));
            }
            if let Some(bx) = g0.get("box").and_then(|v| v.as_str()) {
                meta.entry("email_box")
                    .or_insert(Value::String(bx.to_string()));
            }
            if let Some(t) = pick_i64(g0.get("time")) {
                meta.entry("email_time").or_insert(Value::from(t));
            }

            // role and kvk flag
            let stats_block = sections
                .iter()
                .find(|s| s.get("STs").is_some() || s.get("Role").is_some());
            if let Some(role) = stats_block
                .and_then(|s| s.get("Role"))
                .and_then(|v| v.as_str())
            {
                meta.entry("email_role")
                    .or_insert(Value::String(role.to_string()));
            }
            let is_kvk = stats_block
                .and_then(|s| s.get("isConquerSeason").and_then(|v| v.as_bool()))
                .unwrap_or(false) as i32;
            meta.entry("is_kvk").or_insert(Value::from(is_kvk));

            // time anchors
            let base_epoch = first_epoch_bts(sections)
                .or_else(|| {
                    sections
                        .iter()
                        .find_map(|s| epoch_seconds_val(s.get("Bts")))
                })
                .unwrap_or(0);
            let base_small = first_small_tickstart(sections).unwrap_or(0);
            let (ts, ets) = if let Some((ts, ets)) = small_tick_pair(group) {
                (ts, ets)
            } else {
                let gba = group_epoch_bts(group).unwrap_or(base_epoch);
                let gea = group_epoch_ets(group).unwrap_or(base_epoch);
                (gba - base_epoch + base_small, gea - base_epoch + base_small)
            };
            let start_date = base_epoch + (ts - base_small);
            let end_date = base_epoch + (ets - base_small);
            meta.entry("start_date").or_insert(Value::from(start_date));
            meta.entry("end_date").or_insert(Value::from(end_date));

            // position
            if let Some(pos) = group.iter().find_map(|s| {
                s.get("Pos")
                    .or_else(|| s.get("Attacks").and_then(|a| a.get("Pos")))
            }) {
                let x = pick_f64(pos.get("X")).unwrap_or(0.0);
                let y = pick_f64(pos.get("Y")).unwrap_or(0.0);
                meta.entry("pos_x").or_insert(Value::from(x));
                meta.entry("pos_y").or_insert(Value::from(y));
            } else if let Some(attacks_obj) = sections
                .iter()
                .find_map(|s| s.get("Attacks").filter(|a| a.get(ctx.attack_id).is_some()))
                && let Some(pos) = attacks_obj.get("Pos")
            {
                let x = pick_f64(pos.get("X")).unwrap_or(0.0);
                let y = pick_f64(pos.get("Y")).unwrap_or(0.0);
                meta.entry("pos_x").or_insert(Value::from(x));
                meta.entry("pos_y").or_insert(Value::from(y));
            }

            // players
            if let Some(sts) = stats_block
                .and_then(|s| s.get("STs"))
                .and_then(|v| v.as_object())
            {
                let cnt = sts.keys().filter(|k| k.as_str() != "-2").count() as i32;
                meta.entry("players").or_insert(Value::from(cnt));
            }

            // email_receiver derived from self pid
            // fetch self snapshot/body
            let self_snap = find_self_snapshot(sections);
            let self_body = find_self_body(sections);
            let pid = self_snap
                .get("PId")
                .and_then(|v| v.as_i64())
                .or_else(|| self_body.pointer("/SelfChar/PId").and_then(|v| v.as_i64()))
                .unwrap_or(0);
            if pid != 0 {
                meta.entry("email_receiver")
                    .or_insert(Value::String(pid.to_string()));
            }
        }

        Ok(())
    }
}
