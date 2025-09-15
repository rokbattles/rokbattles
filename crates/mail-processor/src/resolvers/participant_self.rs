use crate::{
    helpers::{
        collect_affix_from_hwbs, collect_buffs_from_hwbs, extract_avatar_url,
        find_self_content_root, find_self_snapshot_section, get_or_insert_object, map_put_f64,
        map_put_i32, map_put_i64, map_put_str, parse_f64,
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

    fn find_self_idx(sections: &[Value]) -> Option<usize> {
        sections.iter().position(|s| {
            s.pointer("/body/content/SelfChar").is_some()
                || s.pointer("/content/SelfChar").is_some()
        })
    }

    fn clamp_stars(raw: i64) -> usize {
        if raw <= 0 {
            return 0;
        }
        core::cmp::min(raw, 4) as usize
    }

    fn digits_to_string(d: [u8; 4]) -> String {
        let mut s = String::with_capacity(4);
        s.push((b'0' + d[0]) as char);
        s.push((b'0' + d[1]) as char);
        s.push((b'0' + d[2]) as char);
        s.push((b'0' + d[3]) as char);
        s
    }

    fn push_skill(v: &Value, dst: &mut [u8; 5], cnt: &mut usize) {
        if *cnt >= dst.len() {
            return;
        }
        if let Some(x) = v.get("SkillLevel").and_then(Value::as_i64)
            && (1..=5).contains(&x)
        {
            dst[*cnt] = x as u8;
            *cnt += 1;
        }
    }

    fn section_tracking_key(sec: &Value) -> Option<&str> {
        sec.get("CTK")
            .and_then(Value::as_str)
            .or_else(|| sec.pointer("/body/content/CTK").and_then(Value::as_str))
            .or_else(|| sec.pointer("/content/CTK").and_then(Value::as_str))
    }

    fn section_content_root(sec: &Value) -> &Value {
        sec.pointer("/body/content")
            .or_else(|| sec.get("content"))
            .unwrap_or(sec)
    }

    fn tracking_key_belongs_to_pid(ctk: &str, pid: i64) -> bool {
        if ctk.is_empty() {
            return false;
        }
        if let Some(pos) = ctk.as_bytes().iter().position(|&b| b == b'_') {
            ctk[..pos].parse::<i64>().ok() == Some(pid)
        } else {
            ctk.parse::<i64>().ok() == Some(pid)
        }
    }

    fn has_self_related_fields(content: &Value) -> bool {
        content.get("HEq").is_some()
            || content.get("CastlePos").is_some()
            || content.get("COSId").is_some()
    }

    fn is_self_section_for_pid(sec: &Value, pid_opt: Option<i64>) -> bool {
        if let Some(pid) = pid_opt {
            if let Some(ctk) = Self::section_tracking_key(sec)
                && Self::tracking_key_belongs_to_pid(ctk, pid)
            {
                return true;
            }
            let content = Self::section_content_root(sec);
            return content.get("PId").and_then(Value::as_i64) == Some(pid);
        }
        Self::section_content_root(sec)
            .pointer("/SelfChar")
            .is_some()
    }

    fn compose_primary_commander_skills_capped(
        sections: &[Value],
        self_body: &Value,
        self_idx: Option<usize>,
        hst_stars: usize,
        pid_opt: Option<i64>,
    ) -> String {
        if hst_stars == 0 {
            return String::new();
        }

        const MAX_DIGITS_TO_OBSERVE: usize = 5;
        let mut observed: [u8; MAX_DIGITS_TO_OBSERVE] = [0; MAX_DIGITS_TO_OBSERVE];
        let mut count = 0usize;

        if let Some(hss) = self_body.pointer("/SelfChar/HSS") {
            Self::push_skill(hss, &mut observed, &mut count);
        }
        Self::push_skill(
            self_body.pointer("/SelfChar").unwrap_or(&Value::Null),
            &mut observed,
            &mut count,
        );
        Self::push_skill(self_body, &mut observed, &mut count);

        if let Some(i) = self_idx {
            if let Some(body) = sections[i].pointer("/body") {
                Self::push_skill(body, &mut observed, &mut count);
            }
            Self::push_skill(&sections[i], &mut observed, &mut count);

            for s in sections.iter().skip(i + 1) {
                if count >= MAX_DIGITS_TO_OBSERVE {
                    break;
                }
                if !Self::is_self_section_for_pid(s, pid_opt) {
                    continue;
                }
                Self::push_skill(s, &mut observed, &mut count);
            }
        }

        if count >= 5 {
            return "5555".to_string();
        }

        let mut d = [0u8; 4];
        let take = core::cmp::min(hst_stars, 4);
        d[..take.min(count)].copy_from_slice(&observed[..take.min(count)]);
        Self::digits_to_string(d)
    }

    fn compose_secondary_commander_skills_capped(sections: &[Value], hst2_stars: usize) -> String {
        if hst2_stars == 0 {
            return String::new();
        }

        let i = match sections.iter().position(|s| s.get("HSS2").is_some()) {
            Some(i) => i,
            None => return String::new(),
        };

        const MAX_DIGITS_TO_OBSERVE: usize = 5;
        let mut observed: [u8; MAX_DIGITS_TO_OBSERVE] = [0; MAX_DIGITS_TO_OBSERVE];
        let mut count = 0usize;

        if let Some(hss2) = sections[i].get("HSS2") {
            Self::push_skill(hss2, &mut observed, &mut count);
        }
        Self::push_skill(&sections[i], &mut observed, &mut count);
        for s in sections.iter().skip(i + 1) {
            if count >= MAX_DIGITS_TO_OBSERVE {
                break;
            }
            Self::push_skill(s, &mut observed, &mut count);
        }

        if count >= 5 {
            return "5555".to_string();
        }

        let mut d = [0u8; 4];
        let take = core::cmp::min(hst2_stars, 4);
        d[..take.min(count)].copy_from_slice(&observed[..take.min(count)]);
        Self::digits_to_string(d)
    }

    fn select_best_self_snapshot_for_pid<'a>(
        sections: &'a [Value],
        pid: i64,
        self_name_hint: Option<&str>,
    ) -> Option<&'a Value> {
        let mut best_ix: Option<usize> = None;
        let mut best_key = (false, false, false, false);

        for (ix, sec) in sections.iter().enumerate() {
            let content = Self::section_content_root(sec);
            let ctk_ok = Self::section_tracking_key(sec)
                .map(|ctk| Self::tracking_key_belongs_to_pid(ctk, pid))
                .unwrap_or(false);
            let pid_ok = content.get("PId").and_then(Value::as_i64) == Some(pid);
            let name_ok = matches!(
                (self_name_hint, content.get("PName").and_then(Value::as_str)),
                (Some(h), Some(pn)) if !h.is_empty() && h == pn
            );
            let selfish = Self::has_self_related_fields(content);

            let key = (ctk_ok, pid_ok, name_ok, selfish);
            if key > best_key {
                best_key = key;
                best_ix = Some(ix);
            }
        }

        best_ix.map(|ix| Self::section_content_root(&sections[ix]))
    }

    fn pick_self_alliance_ct_formation(sections: &[Value]) -> (Option<String>, i32, i32) {
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

    fn find_secondary_commander_level(
        sections: &[Value],
        self_snap: &Value,
        pid: i64,
        hid2: i32,
    ) -> Option<i32> {
        if let Some(lv2) = self_snap.get("HLv2").and_then(Value::as_i64)
            && lv2 != 0
        {
            return Some(lv2 as i32);
        }

        for sec in sections {
            if let Some(sts) = sec.get("STs").and_then(Value::as_object) {
                for (_k, entry) in sts {
                    if entry.get("PId").and_then(Value::as_i64) == Some(pid)
                        && let Some(lv2) = entry.get("HLv2").and_then(Value::as_i64)
                        && lv2 != 0
                    {
                        return Some(lv2 as i32);
                    }
                }
            }
        }

        if hid2 != 0 {
            let target = hid2 as i64;
            for sec in sections {
                if sec.get("HId2").and_then(Value::as_i64) == Some(target)
                    && let Some(lv2) = sec.get("HLv2").and_then(Value::as_i64)
                    && lv2 != 0
                {
                    return Some(lv2 as i32);
                }
                let content = Self::section_content_root(sec);
                if content.get("HId2").and_then(Value::as_i64) == Some(target)
                    && let Some(lv2) = content.get("HLv2").and_then(Value::as_i64)
                    && lv2 != 0
                {
                    return Some(lv2 as i32);
                }
            }
        }
        None
    }

    fn find_primary_commander_level(
        sections: &[Value],
        self_snap: &Value,
        pid: i64,
        hid: i32,
    ) -> Option<i32> {
        if let Some(lv) = self_snap.get("HLv").and_then(Value::as_i64)
            && lv != 0
        {
            return Some(lv as i32);
        }

        for sec in sections {
            if let Some(sts) = sec.get("STs").and_then(Value::as_object) {
                for (_k, entry) in sts {
                    if entry.get("PId").and_then(Value::as_i64) == Some(pid)
                        && let Some(lv) = entry.get("HLv").and_then(Value::as_i64)
                        && lv != 0
                    {
                        return Some(lv as i32);
                    }
                }
            }
        }

        if hid != 0 {
            let target = hid as i64;
            for sec in sections {
                if sec.get("HId").and_then(Value::as_i64) == Some(target)
                    && let Some(lv) = sec.get("HLv").and_then(Value::as_i64)
                    && lv != 0
                {
                    return Some(lv as i32);
                }
                let content = Self::section_content_root(sec);
                if content.get("HId").and_then(Value::as_i64) == Some(target)
                    && let Some(lv) = content.get("HLv").and_then(Value::as_i64)
                    && lv != 0
                {
                    return Some(lv as i32);
                }
            }
        }
        None
    }

    fn find_selfchar_player_id(sections: &[Value]) -> Option<i64> {
        for sec in sections {
            if let Some(pid) = sec
                .pointer("/body/content/SelfChar/PId")
                .and_then(Value::as_i64)
                .or_else(|| sec.pointer("/content/SelfChar/PId").and_then(Value::as_i64))
            {
                return Some(pid);
            }
        }
        None
    }

    fn find_mail_receiver_player_id(sections: &[Value]) -> Option<i64> {
        for sec in sections {
            if let Some(r) = sec.get("receiver").and_then(Value::as_str)
                && let Some(stripped) = r.strip_prefix("player_")
                && let Ok(pid) = stripped.parse::<i64>()
            {
                return Some(pid);
            }
        }
        None
    }
}

impl Resolver for ParticipantSelfResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let sections = ctx.sections;
        let self_body = find_self_content_root(sections).unwrap_or(&Value::Null);

        let obj = match get_or_insert_object(mail, "self") {
            Value::Object(m) => m,
            _ => unreachable!("self must be an object"),
        };

        obj.insert("is_ranged_tower".into(), Value::Bool(false));

        // player id
        let player_pid = Self::find_selfchar_player_id(sections)
            .or_else(|| self_body.pointer("/SelfChar/PId").and_then(Value::as_i64))
            .or_else(|| Self::find_mail_receiver_player_id(sections));
        map_put_i64(obj, "player_id", player_pid);

        let self_name_hint = self_body.pointer("/PName").and_then(Value::as_str);

        let mut self_snap = find_self_snapshot_section(sections).unwrap_or(&Value::Null);
        if let Some(pid) = player_pid {
            let use_override = match Self::section_tracking_key(self_snap) {
                Some(ctk) => !Self::tracking_key_belongs_to_pid(ctk, pid),
                None => true,
            };
            if use_override {
                if let Some(best) =
                    Self::select_best_self_snapshot_for_pid(sections, pid, self_name_hint)
                {
                    self_snap = best;
                } else {
                    let ok_by_pid = self_snap.get("PId").and_then(Value::as_i64) == Some(pid);
                    let ok_by_name = self_snap
                        .get("PName")
                        .and_then(Value::as_str)
                        .and_then(|pn| self_name_hint.map(|h| pn == h))
                        .unwrap_or(false);
                    if ok_by_pid || ok_by_name {
                        self_snap = Self::section_content_root(self_snap);
                    } else {
                        self_snap = &Value::Null;
                    }
                }
            } else {
                self_snap = Self::section_content_root(self_snap);
            }
        }

        // player name
        if obj.get("player_name").is_none()
            && let Some(pname) = self_snap
                .get("PName")
                .and_then(Value::as_str)
                .or_else(|| self_body.pointer("/PName").and_then(Value::as_str))
        {
            obj.insert("player_name".into(), Value::String(pname.to_owned()));
        }

        // avatar url
        if obj.get("avatar_url").is_none() {
            if let Some(url) = extract_avatar_url(self_body.pointer("/SelfChar/Avatar")) {
                obj.insert("avatar_url".into(), Value::String(url));
            } else if let Some(url) = extract_avatar_url(self_snap.get("Avatar")) {
                obj.insert("avatar_url".into(), Value::String(url));
            }
        }

        // alliance and castle pos
        let mut alliance_abbr: Option<String> = None;
        if let Some(pid) = player_pid {
            let mut abbr_zero: Option<String> = None;
            let mut abbr_any: Option<String> = None;
            for s in sections {
                if let Some(sts) = s.get("STs").and_then(Value::as_object) {
                    for (k, entry) in sts {
                        if entry.get("PId").and_then(Value::as_i64) == Some(pid)
                            && let Some(a) = entry.get("Abbr").and_then(Value::as_str)
                            && !a.is_empty()
                        {
                            if k == "0" {
                                abbr_zero.get_or_insert_with(|| a.to_owned());
                            } else if abbr_any.is_none() {
                                abbr_any = Some(a.to_owned());
                            }
                        }
                    }
                }
            }
            alliance_abbr = abbr_zero.or(abbr_any);
        }
        let (abbr_guess, _ct, fm_guess) = Self::pick_self_alliance_ct_formation(sections);
        if alliance_abbr.is_none() {
            alliance_abbr = abbr_guess;
        }
        let final_abbr = alliance_abbr.unwrap_or_default();
        obj.insert("alliance_tag".into(), Value::String(final_abbr.clone()));

        if let Some(castle) = self_snap.get("CastlePos") {
            map_put_f64(obj, "castle_x", parse_f64(castle.get("X")));
            map_put_f64(obj, "castle_y", parse_f64(castle.get("Y")));
        }
        if (obj.get("castle_x").is_none() || obj.get("castle_y").is_none())
            && !final_abbr.is_empty()
        {
            let want_abbr = final_abbr.as_str();
            if let Some(sec) = sections.iter().find(|s| {
                s.get("AName").is_some()
                    && s.get("Abbr")
                        .and_then(Value::as_str)
                        .map(|a| a == want_abbr)
                        .unwrap_or(false)
                    && s.get("CastlePos").is_some()
            }) && let Some(castle) = sec.get("CastlePos")
            {
                map_put_f64(obj, "castle_x", parse_f64(castle.get("X")));
                map_put_f64(obj, "castle_y", parse_f64(castle.get("Y")));
            }
        }
        if (obj.get("castle_x").is_none() || obj.get("castle_y").is_none())
            && !final_abbr.is_empty()
            && let Some(sec) = sections.iter().find(|s| {
                s.get("Abbr")
                    .and_then(Value::as_str)
                    .map(|a| a == final_abbr)
                    .unwrap_or(false)
                    && s.get("CastlePos").is_some()
            })
            && let Some(castle) = sec.get("CastlePos")
        {
            map_put_f64(obj, "castle_x", parse_f64(castle.get("X")));
            map_put_f64(obj, "castle_y", parse_f64(castle.get("Y")));
        }

        // rally
        if let Some(true) = self_snap.get("IsRally").and_then(Value::as_bool) {
            obj.insert("is_rally".into(), Value::from(1));
        }

        // primary commander
        let primary_id = self_body.pointer("/SelfChar/HId").and_then(Value::as_i64);
        let mut primary_lvl = self_snap.get("HLv").and_then(Value::as_i64).unwrap_or(0) as i32;
        if let (Some(pid), Some(hid)) = (
            obj.get("player_id").and_then(Value::as_i64),
            primary_id.map(|x| x as i32),
        ) && primary_lvl == 0
            && let Some(found) = Self::find_primary_commander_level(sections, self_snap, pid, hid)
        {
            primary_lvl = found;
        }

        if primary_id.is_some() || primary_lvl != 0 {
            let mut cmd = Map::new();
            map_put_i32(&mut cmd, "id", primary_id.map(|x| x as i32));
            map_put_i32(&mut cmd, "level", Some(primary_lvl));

            let self_idx = Self::find_self_idx(sections);
            let hst_stars = sections
                .iter()
                .find_map(|s| s.get("HSt").and_then(Value::as_i64))
                .map(Self::clamp_stars)
                .unwrap_or(0);

            let hss = Self::compose_primary_commander_skills_capped(
                sections,
                self_body,
                self_idx,
                hst_stars,
                obj.get("player_id").and_then(Value::as_i64),
            );
            if !hss.is_empty() {
                cmd.insert("skills".into(), Value::String(hss));
            }

            obj.insert("primary_commander".into(), Value::Object(cmd));
        }

        // secondary commander
        let hid2 = self_snap.get("HId2").and_then(Value::as_i64).unwrap_or(0) as i32;

        let mut hlv2 = self_snap.get("HLv2").and_then(Value::as_i64).unwrap_or(0) as i32;
        if let Some(pid) = obj.get("player_id").and_then(Value::as_i64)
            && hlv2 == 0
            && let Some(found) =
                Self::find_secondary_commander_level(sections, self_snap, pid, hid2)
        {
            hlv2 = found;
        }

        let mut cmd2 = Map::new();
        cmd2.insert("id".into(), Value::from(hid2));
        cmd2.insert(
            "level".into(),
            Value::from(if hid2 == 0 { 0 } else { hlv2 }),
        );
        if hid2 != 0 {
            let hst2_stars = sections
                .iter()
                .find_map(|s| s.get("HSt2").and_then(Value::as_i64))
                .map(Self::clamp_stars)
                .unwrap_or(0);
            let hss2 = Self::compose_secondary_commander_skills_capped(sections, hst2_stars);
            if !hss2.is_empty() {
                cmd2.insert("skills".into(), Value::String(hss2));
            }
        }
        obj.insert("secondary_commander".into(), Value::Object(cmd2));

        // kingdom and tracking key
        map_put_i32(
            obj,
            "kingdom_id",
            self_snap
                .get("COSId")
                .and_then(Value::as_i64)
                .map(|x| x as i32),
        );

        // equipment and formation
        map_put_str(
            obj,
            "equipment",
            self_snap.get("HEq").and_then(Value::as_str),
        );
        if let Some(fm) = self_snap.get("HFMs").and_then(Value::as_i64) {
            obj.insert("formation".into(), Value::from(fm as i32));
        } else if fm_guess != 0 && obj.get("formation").is_none() {
            obj.insert("formation".into(), Value::from(fm_guess));
        }

        // armaments and inscriptions
        if let Some(hwbs) = self_snap.get("HWBs") {
            let buffs = collect_buffs_from_hwbs(Some(hwbs));
            if !buffs.is_empty() {
                obj.insert("armament_buffs".into(), Value::String(buffs));
            }
            let insc = collect_affix_from_hwbs(Some(hwbs));
            if !insc.is_empty() {
                obj.insert("inscriptions".into(), Value::String(insc));
            }
        }

        Ok(())
    }
}
