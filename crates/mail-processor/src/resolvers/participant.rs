use crate::{
    helpers::*,
    resolvers::{Resolver, ResolverContext},
};
use serde_json::{Value, json};

pub struct ParticipantResolver;

impl Default for ParticipantResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticipantResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for ParticipantResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let sections = ctx.sections;
        let group = ctx.group;

        // self
        {
            let self_obj = get_or_insert_object(mail, "self");
            let self_snap = find_self_snapshot(sections);
            let self_body = find_self_body(sections);

            if let Some(obj) = self_obj.as_object_mut() {
                if let Some(pid) = self_snap
                    .get("PId")
                    .and_then(|v| v.as_i64())
                    .or_else(|| self_body.pointer("/SelfChar/PId").and_then(|v| v.as_i64()))
                {
                    obj.insert("player_id".to_string(), Value::from(pid));
                }
                if let Some(pname) = self_snap
                    .get("PName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        self_body
                            .pointer("/PName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                {
                    obj.insert("player_name".to_string(), Value::String(pname));
                }

                let (abbr, _ct, formation) = pick_self_abbr_ct_form(sections);
                if !abbr.is_empty() {
                    obj.insert("alliance_tag".to_string(), Value::String(abbr));
                }
                if formation != 0 {
                    obj.insert("formation".to_string(), Value::from(formation));
                }

                if let Some(castle) = self_snap.get("CastlePos") {
                    if let Some(x) = pick_f64(castle.get("X")) {
                        obj.insert("castle_x".to_string(), Value::from(x));
                    }
                    if let Some(y) = pick_f64(castle.get("Y")) {
                        obj.insert("castle_y".to_string(), Value::from(y));
                    }
                }

                let is_rally = self_snap
                    .get("IsRally")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false) as i32;
                if is_rally != 0 {
                    obj.insert("is_rally".to_string(), Value::from(is_rally));
                }

                let hid = self_body
                    .pointer("/SelfChar/HId")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32);
                let hlv = self_snap
                    .get("HLv")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32);
                let hss = compose_hss_mailwide(sections, &self_body);
                if hid.is_some() || hlv.is_some() || !hss.is_empty() {
                    let mut cmd = json!({});
                    if let Some(id) = hid {
                        cmd["id"] = Value::from(id);
                    }
                    if let Some(lv) = hlv {
                        cmd["level"] = Value::from(lv);
                    }
                    if !hss.is_empty() {
                        cmd["skills"] = Value::String(hss);
                    }
                    obj.insert("primary_commander".to_string(), cmd);
                }
                let hid2 = self_snap
                    .get("HId2")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32);
                let hlv2 = pick_hlv2(sections, &self_snap);
                let hss2 = pick_hss2_fourdigits(sections);
                if hid2.is_some() || hlv2 != 0 || !hss2.is_empty() {
                    let mut cmd2 = json!({});
                    if let Some(id) = hid2 {
                        cmd2["id"] = Value::from(id);
                    }
                    if hlv2 != 0 {
                        cmd2["level"] = Value::from(hlv2);
                    }
                    if !hss2.is_empty() {
                        cmd2["skills"] = Value::String(hss2);
                    }
                    obj.insert("secondary_commander".to_string(), cmd2);
                }

                if let Some(k) = self_snap
                    .get("COSId")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32)
                {
                    obj.insert("kingdom_id".to_string(), Value::from(k));
                }
                if let Some(ctk) = self_snap.get("CTK").and_then(|v| v.as_str())
                    && !ctk.is_empty()
                {
                    obj.insert("tracking_key".to_string(), Value::String(ctk.to_string()));
                }

                if let Some(eq) = self_snap.get("HEq").and_then(|v| v.as_str()) {
                    obj.insert("equipment".to_string(), Value::String(eq.to_string()));
                }

                if let Some(fm) = self_snap
                    .get("HFMs")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32)
                {
                    obj.insert("formation".to_string(), Value::from(fm));
                }
                let buffs = join_buffs(self_snap.get("HWBs"));
                if !buffs.is_empty() {
                    obj.insert("armament_buffs".to_string(), Value::String(buffs));
                }
                let inscriptions = join_affix(self_snap.get("HWBs"));
                if !inscriptions.is_empty() {
                    obj.insert("inscriptions".to_string(), Value::String(inscriptions));
                }
            }
        }

        // enemy
        {
            let enemy_obj = get_or_insert_object(mail, "enemy");
            let (_idx_in_group, atk_block) = find_best_attack_block(group, ctx.attack_id);
            let c_idt = atk_block.get("CIdt").cloned().unwrap_or(Value::Null);
            let enemy_pid = c_idt.get("PId").and_then(|v| v.as_i64()).unwrap_or(-2);
            let (enemy_ctid, enemy_abbr, enemy_ct, enemy_pname) =
                pick_enemy_from_ots(group, enemy_pid);
            let enemy_snap = find_enemy_snapshot_for_attack(group, ctx.attack_id)
                .or_else(|| find_enemy_snapshot_by_ctid(group, enemy_ctid))
                .unwrap_or(Value::Null);
            let attack_cid = ctx.attack_id.parse::<i64>().unwrap_or(0);

            if let Some(obj) = enemy_obj.as_object_mut() {
                if let Some(pid) = c_idt.get("PId").and_then(|v| v.as_i64()).or_else(|| {
                    group_ots_entry(group, enemy_ctid)
                        .and_then(|o| o.get("PId"))
                        .and_then(|v| v.as_i64())
                }) {
                    obj.insert("player_id".to_string(), Value::from(pid));
                }
                let pname = if !enemy_pname.trim().is_empty() {
                    Some(enemy_pname)
                } else {
                    enemy_snap
                        .get("PName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                };
                if let Some(name) = pname {
                    obj.insert("player_name".to_string(), Value::String(name));
                }

                if !enemy_abbr.trim().is_empty() {
                    obj.insert("alliance_tag".to_string(), Value::String(enemy_abbr));
                }

                if let Some(castle) = enemy_snap.get("CastlePos") {
                    if let Some(x) = pick_f64(castle.get("X")) {
                        obj.insert("castle_x".to_string(), Value::from(x));
                    }
                    if let Some(y) = pick_f64(castle.get("Y")) {
                        obj.insert("castle_y".to_string(), Value::from(y));
                    }
                }

                let ctk = enemy_snap
                    .get("CTK")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let is_rally = if enemy_snap
                    .get("IsRally")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                    || ctk.contains("rally")
                {
                    1
                } else {
                    0
                };
                if is_rally != 0 {
                    obj.insert("is_rally".to_string(), Value::from(is_rally));
                }

                // Set NPC type info from the attack block when present
                if let Some(nt) = atk_block
                    .get("NpcType")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32)
                {
                    obj.insert("npc_type".to_string(), Value::from(nt));
                }
                if let Some(nbt) = atk_block
                    .get("NpcBType")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32)
                {
                    obj.insert("npc_btype".to_string(), Value::from(nbt));
                }

                let hid = c_idt
                    .get("HId")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        group_ots_entry(group, enemy_ctid)
                            .and_then(|o| o.get("HId"))
                            .and_then(|v| v.as_i64())
                    })
                    .map(|x| x as i32);
                let hlv = c_idt
                    .get("HLv")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        find_group_aname_ct(group, enemy_ct)
                            .and_then(|a| a.get("HLv"))
                            .and_then(|v| v.as_i64())
                    })
                    .or_else(|| {
                        group_ots_entry(group, enemy_ctid)
                            .and_then(|o| o.get("HLv"))
                            .and_then(|v| v.as_i64())
                    })
                    .map(|x| x as i32);
                if hid.is_some() || hlv.is_some() {
                    let mut cmd = json!({});
                    if let Some(id) = hid {
                        cmd["id"] = Value::from(id);
                    }
                    if let Some(lv) = hlv {
                        cmd["level"] = Value::from(lv);
                    }
                    let hss = compose_enemy_hss(group, attack_cid, &c_idt, &enemy_snap);
                    if !hss.is_empty() {
                        cmd["skills"] = Value::String(hss);
                    }
                    obj.insert("primary_commander".to_string(), cmd);
                }
                let hid2 = c_idt
                    .get("HId2")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        group
                            .iter()
                            .find(|s| s.get("AppUid").is_some())
                            .and_then(|snap| snap.get("HId2"))
                            .and_then(|v| v.as_i64())
                    })
                    .or_else(|| {
                        group_ots_entry(group, enemy_ctid)
                            .and_then(|o| o.get("HId2"))
                            .and_then(|v| v.as_i64())
                    })
                    .map(|x| x as i32);
                let hlv2 = c_idt
                    .get("HLv2")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        find_group_aname_ct(group, enemy_ct)
                            .and_then(|a| a.get("HLv2"))
                            .and_then(|v| v.as_i64())
                    })
                    .or_else(|| {
                        group
                            .iter()
                            .find(|s| s.get("AppUid").is_some())
                            .and_then(|snap| snap.get("HLv2"))
                            .and_then(|v| v.as_i64())
                    })
                    .or_else(|| {
                        group_ots_entry(group, enemy_ctid)
                            .and_then(|o| o.get("HLv2"))
                            .and_then(|v| v.as_i64())
                    })
                    .map(|x| x as i32);
                if hid2.is_some() || hlv2.is_some() {
                    let mut cmd2 = json!({});
                    if let Some(id) = hid2 {
                        cmd2["id"] = Value::from(id);
                    }
                    if let Some(lv) = hlv2 {
                        cmd2["level"] = Value::from(lv);
                    }
                    let hss2 = compose_enemy_hss2(group, &enemy_snap);
                    if !hss2.is_empty() {
                        cmd2["skills"] = Value::String(hss2);
                    }
                    obj.insert("secondary_commander".to_string(), cmd2);
                }

                if let Some(k) = enemy_snap
                    .get("COSId")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32)
                {
                    obj.insert("kingdom_id".to_string(), Value::from(k));
                }
                if let Some(ctk) = enemy_snap.get("CTK").and_then(|v| v.as_str())
                    && !ctk.is_empty()
                {
                    obj.insert("tracking_key".to_string(), Value::String(ctk.to_string()));
                }

                if let Some(eq) = enemy_snap.get("HEq").and_then(|v| v.as_str()) {
                    obj.insert("equipment".to_string(), Value::String(eq.to_string()));
                }

                if let Some(fm) = enemy_snap
                    .get("HFMs")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32)
                {
                    obj.insert("formation".to_string(), Value::from(fm));
                }
                let buffs = join_buffs(enemy_snap.get("HWBs"));
                if !buffs.is_empty() {
                    obj.insert("armament_buffs".to_string(), Value::String(buffs));
                }
                let inscriptions = join_affix(enemy_snap.get("HWBs"));
                if !inscriptions.is_empty() {
                    obj.insert("inscriptions".to_string(), Value::String(inscriptions));
                }
            }
        }

        Ok(())
    }
}
