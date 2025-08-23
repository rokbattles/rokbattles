use crate::{
    helpers::*,
    resolvers::{Resolver, ResolverContext},
};
use serde_json::{Value, json};

pub struct ParticipantSelfResolver;

impl Default for ParticipantSelfResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticipantSelfResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for ParticipantSelfResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let sections = ctx.sections;
        let _group = ctx.group;

        // self
        {
            let self_obj = get_or_insert_object(mail, "self");
            let self_snap = find_self_snapshot(sections);
            let self_body = find_self_body(sections);

            if let Some(obj) = self_obj.as_object_mut() {
                // player id & name
                let player_pid = self_snap
                    .get("PId")
                    .and_then(|v| v.as_i64())
                    .or_else(|| self_body.pointer("/SelfChar/PId").and_then(|v| v.as_i64()));
                if let Some(pid) = player_pid {
                    obj.insert("player_id".to_string(), Value::from(pid));
                }

                let mut pname_opt = self_snap
                    .get("PName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        self_body
                            .pointer("/PName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });

                if pname_opt.is_none()
                    && let Some(pid) = player_pid
                {
                    for sec in sections.iter() {
                        if let Some(sts) = sec.get("STs").and_then(|v| v.as_object()) {
                            for (_k, entry) in sts.iter() {
                                if entry.get("PId").and_then(|v| v.as_i64()) == Some(pid)
                                    && let Some(pn) = entry.get("PName").and_then(|v| v.as_str())
                                {
                                    pname_opt = Some(pn.to_string());
                                    break;
                                }
                            }
                            if pname_opt.is_some() {
                                break;
                            }
                        }
                    }
                }

                if pname_opt.is_none()
                    && let Some(pid) = player_pid
                {
                    let prefix = format!("{}_", pid);
                    for sec in sections.iter() {
                        if let Some(ctk) = sec.get("CTK").and_then(|v| v.as_str())
                            && ctk.starts_with(&prefix)
                            && let Some(pn) = sec.get("PName").and_then(|v| v.as_str())
                        {
                            pname_opt = Some(pn.to_string());
                            break;
                        }
                    }
                }

                if let Some(pname) = pname_opt {
                    obj.insert("player_name".to_string(), Value::String(pname));
                }

                // alliance
                let (abbr, _ct, formation) = pick_self_abbr_ct_form(sections);
                if !abbr.is_empty() {
                    obj.insert("alliance_tag".to_string(), Value::String(abbr));
                }
                if formation != 0 {
                    obj.insert("formation".to_string(), Value::from(formation));
                }

                // castle pos
                if let Some(castle) = self_snap.get("CastlePos") {
                    if let Some(x) = pick_f64(castle.get("X")) {
                        obj.insert("castle_x".to_string(), Value::from(x));
                    }
                    if let Some(y) = pick_f64(castle.get("Y")) {
                        obj.insert("castle_y".to_string(), Value::from(y));
                    }
                }

                if (obj.get("castle_x").is_none() || obj.get("castle_y").is_none())
                    && let Some(pid) = player_pid
                {
                    let prefix = format!("{}_", pid);
                    for sec in sections.iter() {
                        if let Some(ctk) = sec.get("CTK").and_then(|v| v.as_str())
                            && ctk.starts_with(&prefix)
                        {
                            if let Some(castle) = sec.get("CastlePos") {
                                if let Some(x) = pick_f64(castle.get("X")) {
                                    obj.insert("castle_x".to_string(), Value::from(x));
                                }
                                if let Some(y) = pick_f64(castle.get("Y")) {
                                    obj.insert("castle_y".to_string(), Value::from(y));
                                }
                            }
                            break;
                        }
                    }
                }

                // rally
                let is_rally = self_snap
                    .get("IsRally")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false) as i32;
                if is_rally != 0 {
                    obj.insert("is_rally".to_string(), Value::from(is_rally));
                }

                // commanders
                let hid = self_body
                    .pointer("/SelfChar/HId")
                    .and_then(|v| v.as_i64())
                    .map(|x| x as i32);
                let hlv = self_snap
                    .get("HLv")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        sections
                            .iter()
                            .find_map(|s| s.get("HLv").and_then(|v| v.as_i64()))
                    })
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
                    .or_else(|| {
                        let (_, h2) =
                            parse_hids_from_ctk(self_snap.get("CTK").and_then(|v| v.as_str()));
                        h2
                    })
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

                // kingdom
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

                // equipment
                if let Some(eq) = self_snap.get("HEq").and_then(|v| v.as_str()) {
                    obj.insert("equipment".to_string(), Value::String(eq.to_string()));
                }

                // formation & armaments & inscriptions
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

        Ok(())
    }
}
