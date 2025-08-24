use crate::{
    helpers::{
        find_self_body_ref, find_self_snapshot_ref, get_or_insert_object, join_affix, join_buffs,
        pick_f64,
    },
    resolvers::{Resolver, ResolverContext},
};
use serde_json::{Map, Value};

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

    fn put_i64(obj: &mut Map<String, Value>, k: &str, v: Option<i64>) {
        if let Some(x) = v {
            obj.insert(k.to_string(), Value::from(x));
        }
    }

    fn put_i32(obj: &mut Map<String, Value>, k: &str, v: Option<i32>) {
        if let Some(x) = v {
            obj.insert(k.to_string(), Value::from(x));
        }
    }

    fn put_f64(obj: &mut Map<String, Value>, k: &str, v: Option<f64>) {
        if let Some(x) = v {
            obj.insert(k.to_string(), Value::from(x));
        }
    }

    fn put_str(obj: &mut Map<String, Value>, k: &str, v: Option<&str>) {
        if let Some(s) = v {
            obj.insert(k.to_string(), Value::String(s.to_owned()));
        }
    }

    fn get_i64_at(v: &Value, ptr: &str) -> Option<i64> {
        v.pointer(ptr).and_then(Value::as_i64)
    }

    fn parse_hids_from_ctk(ctk: Option<&str>) -> (Option<i64>, Option<i64>) {
        if let Some(ctk) = ctk {
            let mut it = ctk.split('_');
            let h1 = it.nth(2).and_then(|s| s.parse().ok());
            let h2 = it.next().and_then(|s| s.parse().ok());
            return (h1, h2);
        }
        (None, None)
    }

    fn pick_hlv2(sections: &[Value], self_snap: &Value) -> i32 {
        self_snap
            .get("HLv2")
            .and_then(Value::as_i64)
            .or_else(|| {
                sections
                    .iter()
                    .find_map(|s| s.get("HLv2").and_then(Value::as_i64))
            })
            .unwrap_or(0) as i32
    }

    fn find_self_idx(sections: &[Value]) -> Option<usize> {
        sections.iter().position(|s| {
            s.pointer("/body/content/SelfChar").is_some()
                || s.pointer("/content/SelfChar").is_some()
        })
    }

    fn pick_hss2_fourdigits(sections: &[Value]) -> String {
        let i = match sections.iter().position(|s| s.get("HSS2").is_some()) {
            Some(i) => i,
            None => return String::new(),
        };

        let mut out = [0i64; 4];
        let mut len = 0usize;

        if let Some(x) = sections[i]
            .get("HSS2")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(Value::as_i64)
        {
            out[len] = x;
            len += 1;
        }
        if let Some(x) = sections[i].get("SkillLevel").and_then(Value::as_i64)
            && len < 4
        {
            out[len] = x;
            len += 1;
        }
        for s in sections.iter().skip(i + 1) {
            if len >= 4 {
                break;
            }
            if let Some(x) = s.get("SkillLevel").and_then(Value::as_i64) {
                out[len] = x;
                len += 1;
            }
        }
        if len == 0 {
            return String::new();
        }

        let hst2 = sections
            .iter()
            .find_map(|s| s.get("HSt2").and_then(Value::as_i64))
            .unwrap_or(0);
        if hst2 >= 6 && out[0] >= 5 {
            for d in out[1..].iter_mut() {
                if *d <= 1 {
                    *d = 5;
                }
            }
        }

        let mut s = String::with_capacity(4);
        for &d in out.iter() {
            use core::fmt::Write;
            let _ = write!(&mut s, "{d}");
        }
        s
    }

    fn compose_hss_mailwide(
        sections: &[Value],
        self_body: &Value,
        self_idx: Option<usize>,
    ) -> String {
        let mut digits = [0i64; 4];
        let mut any = false;

        if let Some(x) = Self::get_i64_at(self_body, "/SelfChar/HSS/SkillLevel") {
            digits[0] = x;
            any = true;
        }
        if let Some(x) = Self::get_i64_at(self_body, "/SelfChar/SkillLevel") {
            digits[1] = x;
            any = true;
        }
        if let Some(x) = Self::get_i64_at(self_body, "/SkillLevel") {
            digits[2] = x;
            any = true;
        }
        if let Some(i) = self_idx
            && let Some(x) = sections[i]
                .pointer("/body/SkillLevel")
                .and_then(Value::as_i64)
                .or_else(|| sections[i].get("SkillLevel").and_then(Value::as_i64))
        {
            digits[3] = x;
            any = true;
        }
        if !any {
            return String::new();
        }

        let hst = sections
            .iter()
            .find_map(|s| s.get("HSt").and_then(Value::as_i64))
            .unwrap_or(0);
        if hst >= 6 && digits[0] >= 5 {
            for d in digits[1..].iter_mut() {
                if *d <= 1 {
                    *d = 5;
                }
            }
        }

        let mut s = String::with_capacity(4);
        for &d in digits.iter() {
            use core::fmt::Write;
            let _ = write!(&mut s, "{d}");
        }
        s
    }

    fn pick_self_abbr_ct_form(sections: &[Value]) -> (Option<String>, i32, i32) {
        if let Some(b) = sections.iter().find(|s| s.get("AName").is_some()) {
            let abbr = b.get("Abbr").and_then(Value::as_str).map(|s| s.to_owned());
            let ct = b.get("CT").and_then(Value::as_i64).unwrap_or(0) as i32;
            let fm = b.get("HFMs").and_then(Value::as_i64).unwrap_or(0) as i32;
            return (abbr, ct, fm);
        }
        if let Some(sts0) = sections
            .iter()
            .find_map(|s| s.get("STs").and_then(|m| m.get("0")))
        {
            let abbr = sts0
                .get("Abbr")
                .and_then(Value::as_str)
                .map(|s| s.to_owned());
            let ct = sts0.get("CT").and_then(Value::as_i64).unwrap_or(0) as i32;
            let fm = sections
                .iter()
                .find_map(|s| s.get("HFMs").and_then(Value::as_i64))
                .unwrap_or(0) as i32;
            return (abbr, ct, fm);
        }
        (
            sections
                .iter()
                .find_map(|s| s.get("Abbr").and_then(Value::as_str))
                .map(|s| s.to_owned()),
            sections
                .iter()
                .find_map(|s| s.get("CT").and_then(Value::as_i64))
                .unwrap_or(0) as i32,
            sections
                .iter()
                .find_map(|s| s.get("HFMs").and_then(Value::as_i64))
                .unwrap_or(0) as i32,
        )
    }

    fn resolve_name_by_pid(sections: &[Value], pid: i64) -> Option<String> {
        for sec in sections {
            if let Some(sts) = sec.get("STs").and_then(Value::as_object) {
                for (_k, entry) in sts {
                    if entry.get("PId").and_then(Value::as_i64) == Some(pid)
                        && let Some(pn) = entry.get("PName").and_then(Value::as_str)
                    {
                        return Some(pn.to_owned());
                    }
                }
            }
        }
        None
    }

    fn resolve_name_by_ctk_prefix(sections: &[Value], pid: i64) -> Option<String> {
        let prefix = {
            let mut s = itoa::Buffer::new();
            let pid_str = s.format(pid);
            let mut owned = String::with_capacity(pid_str.len() + 1);
            owned.push_str(pid_str);
            owned.push('_');
            owned
        };
        for sec in sections {
            if let Some(ctk) = sec.get("CTK").and_then(Value::as_str)
                && ctk.starts_with(&prefix)
                && let Some(pn) = sec.get("PName").and_then(Value::as_str)
            {
                return Some(pn.to_owned());
            }
        }
        None
    }
}

impl Resolver for ParticipantSelfResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let sections = ctx.sections;
        let self_snap = find_self_snapshot_ref(sections).unwrap_or(&Value::Null);
        let self_body = find_self_body_ref(sections).unwrap_or(&Value::Null);
        let self_idx = Self::find_self_idx(sections);

        let obj = match get_or_insert_object(mail, "self") {
            Value::Object(m) => m,
            _ => unreachable!("self must be an object"),
        };

        // player id
        let player_pid = self_snap
            .get("PId")
            .and_then(Value::as_i64)
            .or_else(|| Self::get_i64_at(self_body, "/SelfChar/PId"));
        Self::put_i64(obj, "player_id", player_pid);

        // player name
        if obj.get("player_name").is_none()
            && let Some(pname) = self_snap
                .get("PName")
                .and_then(Value::as_str)
                .or_else(|| self_body.pointer("/PName").and_then(Value::as_str))
                .map(|s| s.to_owned())
                .or_else(|| player_pid.and_then(|pid| Self::resolve_name_by_pid(sections, pid)))
                .or_else(|| {
                    player_pid.and_then(|pid| Self::resolve_name_by_ctk_prefix(sections, pid))
                })
        {
            obj.insert("player_name".into(), Value::String(pname));
        }

        // alliance
        let (abbr_opt, _ct, fm_guess) = Self::pick_self_abbr_ct_form(sections);
        if let Some(abbr) = abbr_opt {
            obj.insert("alliance_tag".into(), Value::String(abbr));
        }

        // castle pos
        if let Some(castle) = self_snap.get("CastlePos") {
            Self::put_f64(obj, "castle_x", pick_f64(castle.get("X")));
            Self::put_f64(obj, "castle_y", pick_f64(castle.get("Y")));
        }
        if (obj.get("castle_x").is_none() || obj.get("castle_y").is_none())
            && let Some(pid) = player_pid
        {
            let mut b = itoa::Buffer::new();
            let s = b.format(pid);
            let mut prefix = String::with_capacity(s.len() + 1);
            prefix.push_str(s);
            prefix.push('_');

            if let Some(castle) = sections.iter().find_map(|sec| {
                sec.get("CTK")
                    .and_then(Value::as_str)
                    .filter(|ctk| ctk.starts_with(&prefix))
                    .and_then(|_| sec.get("CastlePos"))
            }) {
                Self::put_f64(obj, "castle_x", pick_f64(castle.get("X")));
                Self::put_f64(obj, "castle_y", pick_f64(castle.get("Y")));
            }
        }

        // rally
        if let Some(b) = self_snap.get("IsRally").and_then(Value::as_bool)
            && b
        {
            obj.insert("is_rally".into(), Value::from(1));
        }

        // commanders
        let hid = Self::get_i64_at(self_body, "/SelfChar/HId").map(|x| x as i32);
        let hlv = self_snap
            .get("HLv")
            .and_then(Value::as_i64)
            .or_else(|| {
                sections
                    .iter()
                    .find_map(|s| s.get("HLv").and_then(Value::as_i64))
            })
            .map(|x| x as i32);
        let hss = Self::compose_hss_mailwide(sections, self_body, self_idx);

        if hid.is_some() || hlv.is_some() || !hss.is_empty() {
            let mut cmd = Map::new();
            Self::put_i32(&mut cmd, "id", hid);
            Self::put_i32(&mut cmd, "level", hlv);
            if !hss.is_empty() {
                cmd.insert("skills".into(), Value::String(hss));
            }
            obj.insert("primary_commander".into(), Value::Object(cmd));
        }

        let hid2 = self_snap
            .get("HId2")
            .and_then(Value::as_i64)
            .or_else(|| Self::parse_hids_from_ctk(self_snap.get("CTK").and_then(Value::as_str)).1)
            .map(|x| x as i32);
        let hlv2 = Self::pick_hlv2(sections, self_snap);
        let hss2 = Self::pick_hss2_fourdigits(sections);

        if hid2.is_some() || hlv2 != 0 || !hss2.is_empty() {
            let mut cmd2 = Map::new();
            Self::put_i32(&mut cmd2, "id", hid2);
            if hlv2 != 0 {
                cmd2.insert("level".into(), Value::from(hlv2));
            }
            if !hss2.is_empty() {
                cmd2.insert("skills".into(), Value::String(hss2));
            }
            obj.insert("secondary_commander".into(), Value::Object(cmd2));
        }

        // kingdom & tracking key
        Self::put_i32(
            obj,
            "kingdom_id",
            self_snap
                .get("COSId")
                .and_then(Value::as_i64)
                .map(|x| x as i32),
        );
        if let Some(ctk) = self_snap
            .get("CTK")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            obj.insert("tracking_key".into(), Value::String(ctk.to_owned()));
        }

        // equipment
        Self::put_str(
            obj,
            "equipment",
            self_snap.get("HEq").and_then(Value::as_str),
        );

        // formation, armament buffs, inscriptions
        if let Some(fm) = self_snap
            .get("HFMs")
            .and_then(Value::as_i64)
            .map(|x| x as i32)
        {
            obj.insert("formation".into(), Value::from(fm));
        } else if fm_guess != 0 && obj.get("formation").is_none() {
            obj.insert("formation".into(), Value::from(fm_guess));
        }

        let buffs = join_buffs(self_snap.get("HWBs"));
        if !buffs.is_empty() {
            obj.insert("armament_buffs".into(), Value::String(buffs));
        }

        let insc = join_affix(self_snap.get("HWBs"));
        if !insc.is_empty() {
            obj.insert("inscriptions".into(), Value::String(insc));
        }

        Ok(())
    }
}
