use crate::{
    helpers::{find_best_attack_block, get_or_insert_object, join_affix, join_buffs, pick_f64},
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

    fn find_enemy_snapshot_for_attack(group: &[Value], attack_id: &str) -> Option<Value> {
        for s in group {
            if s.get("AppUid").is_some() {
                let idt_match = s
                    .get("Idt")
                    .and_then(|v| v.as_str())
                    .map(|x| x == attack_id)
                    .unwrap_or(false)
                    || s.get("Idt")
                        .and_then(|v| v.as_i64())
                        .map(|x| x.to_string() == attack_id)
                        .unwrap_or(false);
                let cid_match = s
                    .get("CId")
                    .and_then(|v| v.as_i64())
                    .map(|x| x.to_string() == attack_id)
                    .unwrap_or(false);
                if idt_match || cid_match {
                    return Some(s.clone());
                }
            }
        }
        None
    }

    fn find_enemy_snapshot_by_ctid(group: &[Value], enemy_ctid: i64) -> Option<Value> {
        for s in group {
            if s.get("AppUid").is_some()
                && s.get("CtId").and_then(|v| v.as_i64()).unwrap_or(-1) == enemy_ctid
            {
                return Some(s.clone());
            }
        }
        None
    }

    fn pick_enemy_from_ots(group: &[Value], enemy_pid: i64) -> (i64, String, i32, String) {
        if let Some(ots) = group
            .iter()
            .find_map(|s| s.get("OTs").and_then(|v| v.as_object()))
        {
            for (ctid_str, v) in ots {
                if v.get("PId").and_then(|x| x.as_i64()).unwrap_or(-1) == enemy_pid {
                    let ctid = ctid_str.parse::<i64>().unwrap_or(0);
                    let abbr = v
                        .get("Abbr")
                        .and_then(|x| x.as_str())
                        .unwrap_or(" ")
                        .to_string();
                    let ct = v.get("CT").and_then(|x| x.as_i64()).unwrap_or(1) as i32;
                    let pname = v
                        .get("PName")
                        .and_then(|x| x.as_str())
                        .unwrap_or(" ")
                        .to_string();
                    return (ctid, abbr, ct, pname);
                }
            }
            if let Some((ctid_str, v)) = ots.iter().next() {
                let ctid = ctid_str.parse::<i64>().unwrap_or(0);
                let abbr = v
                    .get("Abbr")
                    .and_then(|x| x.as_str())
                    .unwrap_or(" ")
                    .to_string();
                let ct = v.get("CT").and_then(|x| x.as_i64()).unwrap_or(1) as i32;
                let pname = v
                    .get("PName")
                    .and_then(|x| x.as_str())
                    .unwrap_or(" ")
                    .to_string();
                return (ctid, abbr, ct, pname);
            }
        }
        (0, " ".to_string(), 1, " ".to_string())
    }

    fn find_group_aname_ct(group: &[Value], ct: i32) -> Option<&Value> {
        group.iter().find(|s| {
            s.get("AName").is_some()
                && s.get("CT").and_then(|v| v.as_i64()).unwrap_or(-1) as i32 == ct
        })
    }

    fn group_ots_entry(group: &[Value], ctid: i64) -> Option<&Value> {
        group.iter().find_map(|s| {
            s.get("OTs")
                .and_then(|v| v.as_object())
                .and_then(|o| o.get(&ctid.to_string()))
        })
    }

    fn digits_all_leq_one(s: &str) -> bool {
        s.chars().all(|c| matches!(c, '0' | '1'))
    }

    fn compute_primary_level(c_idt: &Value, enemy_snap: &Value) -> i64 {
        let primary = c_idt
            .get("HSS")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0);

        if primary >= 5 {
            return primary;
        }

        let hst = enemy_snap.get("HSt").and_then(|v| v.as_i64()).unwrap_or(0);
        let hst2 = enemy_snap.get("HSt2").and_then(|v| v.as_i64()).unwrap_or(0);
        if hst >= 6 || hst2 >= 6 {
            return 5;
        }

        primary
    }

    fn normalize_hss(mut hss: String, enemy_snap: &Value, c_idt: &Value) -> String {
        if hss.len() == 4 && Self::digits_all_leq_one(&hss) {
            let prim5 = c_idt
                .get("HSS")
                .and_then(|o| o.get("SkillLevel"))
                .and_then(|v| v.as_i64())
                == Some(5);
            let hst = enemy_snap.get("HSt").and_then(|v| v.as_i64()).unwrap_or(0);
            let hst2 = enemy_snap.get("HSt2").and_then(|v| v.as_i64()).unwrap_or(0);
            if prim5 || hst >= 6 || hst2 >= 6 {
                hss = "5555".to_string();
            }
        }
        hss
    }

    fn normalize_hss2(mut hss2: String, enemy_snap: &Value) -> String {
        if hss2.len() == 4 && Self::digits_all_leq_one(&hss2) {
            let hst = enemy_snap.get("HSt").and_then(|v| v.as_i64()).unwrap_or(0);
            let hst2 = enemy_snap.get("HSt2").and_then(|v| v.as_i64()).unwrap_or(0);
            if hst >= 6 || hst2 >= 6 {
                hss2 = "5555".to_string();
            }
        }
        hss2
    }

    fn find_attack_container(
        group: &[Value],
        attack_cid: i64,
    ) -> Option<(usize, bool, &Value, &Value)> {
        let key = attack_cid.to_string();
        for (i, sec) in group.iter().enumerate() {
            if let Some(attacks) = sec.get("Attacks")
                && let Some(obj) = attacks.get(&key)
            {
                return Some((i, true, attacks, obj));
            }
            if let Some(obj) = sec.get(&key) {
                return Some((i, false, sec, obj));
            }
        }
        None
    }

    fn group_players_count(group: &[Value]) -> usize {
        for s in group {
            if let Some(sts) = s.get("STs").and_then(|v| v.as_object()) {
                return sts.keys().filter(|k| k.as_str() != "-2").count();
            }
        }
        0
    }

    fn compose_enemy_hss_precise(
        group: &[Value],
        attack_cid: i64,
        c_idt: &Value,
    ) -> Option<String> {
        if Self::group_players_count(group) > 1 {
            return None;
        }

        let s1 = c_idt.get("SkillLevel").and_then(|v| v.as_i64())?;
        let s4 = c_idt
            .get("HSS")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(|v| v.as_i64())?;

        let (_idx, is_attacks, container, obj) = Self::find_attack_container(group, attack_cid)?;

        if is_attacks {
            return None;
        }

        let mut pairs: Vec<(i64, i64)> = Vec::new();

        if let Some(id) = obj.get("SkillId").and_then(|v| v.as_i64())
            && let Some(lv) = obj.get("SkillLevel").and_then(|v| v.as_i64())
        {
            pairs.push((id, lv));
        }

        if let Some(id) = container.get("SkillId").and_then(|v| v.as_i64())
            && let Some(lv) = container.get("SkillLevel").and_then(|v| v.as_i64())
        {
            pairs.push((id, lv));
        }

        if pairs.len() < 2 {
            return None;
        }

        let cid_skill_id = c_idt
            .get("SkillId")
            .and_then(|v| v.as_i64())
            .unwrap_or_default();
        let both_mid_high = pairs.iter().all(|(id, _)| *id >= 1000);
        if !(cid_skill_id > 0 && cid_skill_id < 1000 && both_mid_high) {
            return None;
        }

        pairs.sort_by_key(|(id, _)| *id);
        if (pairs[1].0 - pairs[0].0).abs() != 1 {
            return None;
        }
        let s2 = pairs[0].1;
        let s3 = pairs[1].1;

        if s2 >= 5 && s3 >= 5 && s4 < 5 {
            return None;
        }

        Some(format!(
            "{}{}{}{}",
            s1.clamp(0, 9),
            s2.clamp(0, 9),
            s3.clamp(0, 9),
            s4.clamp(0, 9)
        ))
    }

    fn compose_enemy_hss2_precise(group: &[Value], attack_cid: i64) -> Option<String> {
        if Self::group_players_count(group) > 1 {
            return None;
        }
        let mut base_idx: Option<usize> = None;
        let mut s4: Option<i64> = None;
        for (i, sec) in group.iter().enumerate() {
            if sec.get("CId").and_then(|v| v.as_i64()) == Some(attack_cid)
                && let Some(hss2) = sec.get("HSS2")
                && let Some(lv) = hss2.get("SkillLevel").and_then(|v| v.as_i64())
            {
                base_idx = Some(i);
                s4 = Some(lv);
                break;
            }
        }
        let base_idx = base_idx?;
        let s4 = s4?;

        let mut pairs: Vec<(i64, i64)> = Vec::new();
        let base = &group[base_idx];
        if let Some(id) = base.get("SkillId").and_then(|v| v.as_i64())
            && let Some(lv) = base.get("SkillLevel").and_then(|v| v.as_i64())
        {
            pairs.push((id, lv));
        }

        let mut j = base_idx + 1;
        while pairs.len() < 3 && j < group.len() && j < base_idx + 6 {
            let sec = &group[j];
            if let (Some(id), Some(lv)) = (
                sec.get("SkillId").and_then(|v| v.as_i64()),
                sec.get("SkillLevel").and_then(|v| v.as_i64()),
            ) {
                pairs.push((id, lv));
            }
            j += 1;
        }

        if pairs.is_empty() {
            return None;
        }

        let base_id = base.get("SkillId").and_then(|v| v.as_i64())?;
        let mut filtered: Vec<(i64, i64)> = pairs
            .into_iter()
            .filter(|(id, _)| (id / 100) == (base_id / 100) || (id - base_id).abs() <= 10)
            .collect();

        if filtered.len() < 3 {
            return None;
        }

        filtered.sort_by_key(|(id, _)| (id - base_id).abs());
        let mut chosen: Vec<(i64, i64)> = filtered.into_iter().take(3).collect();
        chosen.sort_by_key(|(id, _)| *id);

        if !(chosen[1].0 == chosen[0].0 + 1 && chosen[2].0 == chosen[1].0 + 1) {
            return None;
        }

        let s1 = chosen[0].1;
        let s2 = chosen[1].1;
        let s3 = chosen[2].1;

        Some(format!(
            "{}{}{}{}",
            s1.clamp(0, 9),
            s2.clamp(0, 9),
            s3.clamp(0, 9),
            s4.clamp(0, 9)
        ))
    }

    fn compose_enemy_hss(
        group: &[Value],
        attack_cid: i64,
        c_idt: &Value,
        enemy_snap: &Value,
    ) -> String {
        if c_idt.get("PId").and_then(|v| v.as_i64()) != Some(-2)
            && let Some(smart) = Self::compose_enemy_hss_precise(group, attack_cid, c_idt)
        {
            return Self::normalize_hss(smart, enemy_snap, c_idt);
        }

        let primary = Self::compute_primary_level(c_idt, enemy_snap);
        if primary <= 0 {
            return String::new();
        }

        let mut secondary: Option<i64> = group
            .iter()
            .find(|s| s.get("CId").and_then(|v| v.as_i64()) == Some(attack_cid))
            .and_then(|sec| sec.get("HSS2"))
            .and_then(|o| o.get("SkillLevel"))
            .and_then(|x| x.as_i64());

        if secondary.is_none()
            && let Some((i, _)) = group
                .iter()
                .enumerate()
                .find(|(_, s)| s.get("CId").and_then(|v| v.as_i64()) == Some(attack_cid))
        {
            let start = i.saturating_sub(2);
            let end = (i + 2).min(group.len().saturating_sub(1));
            secondary = group.iter().take(end + 1).skip(start).find_map(|v| {
                v.get("HSS2")
                    .and_then(|o| o.get("SkillLevel"))
                    .and_then(|x| x.as_i64())
            });
        }

        // npc
        if c_idt.get("PId").and_then(|v| v.as_i64()) == Some(-2) {
            let s = format!("{}000", primary.clamp(0, 9));
            return Self::normalize_hss(s, enemy_snap, c_idt);
        }

        let tail = secondary.unwrap_or(primary);
        let s = format!(
            "{}{}{}{}",
            primary.clamp(0, 9),
            primary.clamp(0, 9),
            primary.clamp(0, 9),
            tail.clamp(0, 9)
        );

        Self::normalize_hss(s, enemy_snap, c_idt)
    }

    fn compose_enemy_hss2(group: &[Value], enemy_snap: &Value) -> String {
        let lvl = group.iter().find_map(|s| {
            s.get("HSS2")
                .and_then(|o| o.get("SkillLevel"))
                .and_then(|x| x.as_i64())
        });
        match lvl {
            Some(x) => Self::normalize_hss2(format!("{0}{0}{0}{0}", x), enemy_snap),
            None => String::new(),
        }
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
                Self::pick_enemy_from_ots(group, enemy_pid);
            let enemy_snap = Self::find_enemy_snapshot_for_attack(group, ctx.attack_id)
                .or_else(|| Self::find_enemy_snapshot_by_ctid(group, enemy_ctid))
                .unwrap_or(Value::Null);
            let attack_cid = ctx.attack_id.parse::<i64>().unwrap_or(0);

            if let Some(obj) = enemy_obj.as_object_mut() {
                // player id & name
                if let Some(pid) = c_idt.get("PId").and_then(|v| v.as_i64()).or_else(|| {
                    Self::group_ots_entry(group, enemy_ctid)
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
                        Self::group_ots_entry(group, enemy_ctid)
                            .and_then(|o| o.get("HId"))
                            .and_then(|v| v.as_i64())
                    })
                    .map(|x| x as i32);
                let hlv = c_idt
                    .get("HLv")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        Self::find_group_aname_ct(group, enemy_ct)
                            .and_then(|a| a.get("HLv"))
                            .and_then(|v| v.as_i64())
                    })
                    .or_else(|| {
                        Self::group_ots_entry(group, enemy_ctid)
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
                    let hss = Self::compose_enemy_hss(group, attack_cid, &c_idt, &enemy_snap);
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
                        Self::group_ots_entry(group, enemy_ctid)
                            .and_then(|o| o.get("HId2"))
                            .and_then(|v| v.as_i64())
                    })
                    .map(|x| x as i32);
                let hlv2 = c_idt
                    .get("HLv2")
                    .and_then(|v| v.as_i64())
                    .or_else(|| {
                        Self::find_group_aname_ct(group, enemy_ct)
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
                        Self::group_ots_entry(group, enemy_ctid)
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
                        Self::compose_enemy_hss2(group, &enemy_snap)
                    } else {
                        Self::compose_enemy_hss2_precise(group, attack_cid)
                            .unwrap_or_else(|| Self::compose_enemy_hss2(group, &enemy_snap))
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
