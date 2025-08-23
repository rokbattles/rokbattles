use crate::{
    helpers::*,
    resolvers::{Resolver, ResolverContext},
};
use serde_json::{Value, json};

pub struct ParticipantEnemyResolver;

impl Default for ParticipantEnemyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticipantEnemyResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for ParticipantEnemyResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let sections = ctx.sections;
        let group = ctx.group;

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
                // player id & name
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

                // alliance
                if !enemy_abbr.trim().is_empty() {
                    obj.insert("alliance_tag".to_string(), Value::String(enemy_abbr));
                }

                // castle pos
                if let Some(castle) = enemy_snap.get("CastlePos") {
                    if let Some(x) = pick_f64(castle.get("X")) {
                        obj.insert("castle_x".to_string(), Value::from(x));
                    }
                    if let Some(y) = pick_f64(castle.get("Y")) {
                        obj.insert("castle_y".to_string(), Value::from(y));
                    }
                }
                if (obj.get("castle_x").is_none() || obj.get("castle_y").is_none())
                    && let Some(castle) = atk_block.get("CastlePos")
                {
                    if let Some(x) = pick_f64(castle.get("X")) {
                        obj.insert("castle_x".to_string(), Value::from(x));
                    }
                    if let Some(y) = pick_f64(castle.get("Y")) {
                        obj.insert("castle_y".to_string(), Value::from(y));
                    }
                }

                // tracking key & rally
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

                // npc
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

                // commanders
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
                    .or_else(|| atk_block.get("HLv").and_then(|v| v.as_i64()))
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
                    let hss2 = if enemy_snap.get("HSt2").and_then(|v| v.as_i64()).unwrap_or(0) >= 6
                    {
                        compose_enemy_hss2(group, &enemy_snap)
                    } else {
                        compose_enemy_hss2_precise(group, attack_cid)
                            .unwrap_or_else(|| compose_enemy_hss2(group, &enemy_snap))
                    };
                    if !hss2.is_empty() {
                        cmd2["skills"] = Value::String(hss2);
                    }
                    obj.insert("secondary_commander".to_string(), cmd2);
                }

                // kingdom
                if let Some(k) = enemy_snap
                    .get("COSId")
                    .and_then(|v| v.as_i64())
                    .filter(|&x| x != 0)
                    .map(|x| x as i32)
                {
                    obj.insert("kingdom_id".to_string(), Value::from(k));
                } else if let Some(k2) = c_idt
                    .get("COSId")
                    .and_then(|v| v.as_i64())
                    .filter(|&x| x != 0)
                    .map(|x| x as i32)
                {
                    obj.insert("kingdom_id".to_string(), Value::from(k2));
                } else {
                    let k3 = sections
                        .iter()
                        .find_map(|s| s.get("GsId").and_then(|v| v.as_i64()))
                        .or_else(|| {
                            sections
                                .first()
                                .and_then(|s| s.get("serverId").and_then(|v| v.as_i64()))
                        });
                    if let Some(k3v) = k3 {
                        obj.insert("kingdom_id".to_string(), Value::from(k3v as i32));
                    }
                }
                if let Some(ctk) = enemy_snap.get("CTK").and_then(|v| v.as_str())
                    && !ctk.is_empty()
                {
                    obj.insert("tracking_key".to_string(), Value::String(ctk.to_string()));
                } else if let Some(ctk2) = atk_block.get("CTK").and_then(|v| v.as_str())
                    && !ctk2.is_empty()
                {
                    obj.insert("tracking_key".to_string(), Value::String(ctk2.to_string()));
                }

                // equipment
                if let Some(eq) = enemy_snap.get("HEq").and_then(|v| v.as_str()) {
                    obj.insert("equipment".to_string(), Value::String(eq.to_string()));
                }
                if obj.get("equipment").is_none()
                    && let Some(eq2) = atk_block.get("HEq").and_then(|v| v.as_str())
                {
                    obj.insert("equipment".to_string(), Value::String(eq2.to_string()));
                }

                // formation & armaments & inscriptions
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
