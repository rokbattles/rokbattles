use crate::{
    helpers::{
        find_self_body, find_self_snapshot, get_or_insert_object, join_affix, join_buffs, pick_f64,
    },
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

    fn parse_hids_from_ctk(ctk: Option<&str>) -> (Option<i64>, Option<i64>) {
        if let Some(ctk) = ctk {
            let parts: Vec<&str> = ctk.split('_').collect();
            if parts.len() >= 4 {
                let h1 = parts[2].parse::<i64>().ok();
                let h2 = parts[3].parse::<i64>().ok();
                return (h1, h2);
            }
        }
        (None, None)
    }

    fn pick_hlv2(sections: &[Value], self_snap: &Value) -> i32 {
        self_snap
            .get("HLv2")
            .and_then(|v| v.as_i64())
            .or_else(|| {
                sections
                    .iter()
                    .find_map(|s| s.get("HLv2").and_then(|v| v.as_i64()))
            })
            .unwrap_or(0) as i32
    }

    fn pick_hss2_fourdigits(sections: &[Value]) -> String {
        let idx = sections.iter().position(|s| s.get("HSS2").is_some());
        if idx.is_none() {
            return String::new();
        }
        let i = idx.unwrap();

        let mut digits: Vec<i64> = Vec::new();
        if let Some(x) = sections[i]
            .get("HSS2")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(|x| x.as_i64())
        {
            digits.push(x);
        }
        if let Some(x) = sections[i].get("SkillLevel").and_then(|x| x.as_i64()) {
            digits.push(x);
        }
        for s in sections.iter().skip(i + 1) {
            if let Some(x) = s.get("SkillLevel").and_then(|x| x.as_i64()) {
                digits.push(x);
                if digits.len() >= 4 {
                    break;
                }
            }
        }

        if digits.is_empty() {
            return String::new();
        }

        while digits.len() < 4 {
            digits.push(0);
        }
        digits.truncate(4);

        let hst2 = sections
            .iter()
            .find_map(|s| s.get("HSt2").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        if hst2 >= 6 && digits.first().copied().unwrap_or(0) >= 5 {
            for d in digits.iter_mut().skip(1) {
                if *d <= 1 {
                    *d = 5;
                }
            }
        }

        digits
            .into_iter()
            .map(|n| n.to_string())
            .collect::<String>()
    }

    fn compose_hss_mailwide(sections: &[Value], self_body: &Value) -> String {
        let mut digits: [Option<i64>; 4] = [None, None, None, None];

        let self_idx = sections.iter().position(|s| {
            s.pointer("/body/content/SelfChar").is_some()
                || s.pointer("/content/SelfChar").is_some()
        });

        if let Some(x) = self_body
            .pointer("/SelfChar/HSS/SkillLevel")
            .and_then(|v| v.as_i64())
        {
            digits[0] = Some(x);
        }
        if let Some(x) = self_body
            .pointer("/SelfChar/SkillLevel")
            .and_then(|v| v.as_i64())
        {
            digits[1] = Some(x);
        }
        if let Some(x) = self_body.pointer("/SkillLevel").and_then(|v| v.as_i64()) {
            digits[2] = Some(x);
        }
        if let Some(i) = self_idx
            && let Some(x) = sections[i]
                .pointer("/body/SkillLevel")
                .and_then(|v| v.as_i64())
                .or_else(|| sections[i].get("SkillLevel").and_then(|v| v.as_i64()))
        {
            digits[3] = Some(x);
        }

        if digits.iter().all(|d| d.is_none()) {
            return String::new();
        }

        let mut out: Vec<i64> = digits.iter().map(|d| d.unwrap_or(0)).collect::<Vec<_>>();

        let hst = sections
            .iter()
            .find_map(|s| s.get("HSt").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        if hst >= 6 && out.first().copied().unwrap_or(0) >= 5 {
            for d in out.iter_mut().skip(1) {
                if *d <= 1 {
                    *d = 5;
                }
            }
        }

        out.into_iter().map(|n| n.to_string()).collect::<String>()
    }

    fn pick_self_abbr_ct_form(sections: &[Value]) -> (String, i32, i32) {
        if let Some(b) = sections.iter().find(|s| s.get("AName").is_some()) {
            let abbr = b
                .get("Abbr")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ct = b.get("CT").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let fm = b.get("HFMs").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            return (abbr, ct, fm);
        }
        if let Some(sts0) = sections
            .iter()
            .find_map(|s| s.get("STs").and_then(|m| m.get("0")))
        {
            let abbr = sts0
                .get("Abbr")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ct = sts0.get("CT").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let fm = sections
                .iter()
                .find_map(|s| s.get("HFMs").and_then(|v| v.as_i64()))
                .unwrap_or(0) as i32;
            return (abbr, ct, fm);
        }
        (
            sections
                .iter()
                .find_map(|s| s.get("Abbr").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string(),
            sections
                .iter()
                .find_map(|s| s.get("CT").and_then(|v| v.as_i64()))
                .unwrap_or(0) as i32,
            sections
                .iter()
                .find_map(|s| s.get("HFMs").and_then(|v| v.as_i64()))
                .unwrap_or(0) as i32,
        )
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
                let (abbr, _ct, formation) = Self::pick_self_abbr_ct_form(sections);
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
                let hss = Self::compose_hss_mailwide(sections, &self_body);
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
                        let (_, h2) = Self::parse_hids_from_ctk(
                            self_snap.get("CTK").and_then(|v| v.as_str()),
                        );
                        h2
                    })
                    .map(|x| x as i32);
                let hlv2 = Self::pick_hlv2(sections, &self_snap);
                let hss2 = Self::pick_hss2_fourdigits(sections);
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
