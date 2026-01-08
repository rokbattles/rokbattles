use crate::{
    helpers::{
        collect_affix_from_hwbs, collect_buffs_from_hwbs, extract_app_uid,
        extract_app_uid_from_avatar_url, extract_avatar_frame_url, extract_avatar_url,
        find_attack_block_best_match, get_or_insert_object_map, map_put_f64, map_put_i32,
        map_put_i64, map_put_str, parse_f64,
    },
    resolvers::{Resolver, ResolverContext},
};
use serde_json::{Map, Value};

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

    fn parse_attack_identifier(s: &str) -> (Option<i64>, &str) {
        (s.parse::<i64>().ok(), s)
    }

    fn attack_identifier_matches(v: &Value, str_id: &str, num_id: Option<i64>) -> bool {
        v.as_str().map(|x| x == str_id).unwrap_or(false)
            || num_id.is_some_and(|n| v.as_i64() == Some(n))
    }

    fn find_enemy_snapshot_by_identifier_strict<'a>(
        group: &'a [Value],
        str_id: &str,
        num_id: Option<i64>,
    ) -> Option<&'a Value> {
        group.iter().find(|s| {
            s.get("AppUid").is_some()
                && (s
                    .get("Idt")
                    .is_some_and(|v| Self::attack_identifier_matches(v, str_id, num_id))
                    || s.get("CId")
                        .is_some_and(|v| Self::attack_identifier_matches(v, str_id, num_id)))
        })
    }

    fn find_enemy_snapshot_by_identifier_loose<'a>(
        group: &'a [Value],
        str_id: &str,
        num_id: Option<i64>,
    ) -> Option<&'a Value> {
        group.iter().find(|s| {
            s.get("Idt")
                .is_some_and(|v| Self::attack_identifier_matches(v, str_id, num_id))
                || s.get("CId")
                    .is_some_and(|v| Self::attack_identifier_matches(v, str_id, num_id))
        })
    }

    fn find_enemy_snapshot_by_ctid(group: &[Value], enemy_ctid: i64) -> Option<&Value> {
        group.iter().find(|s| {
            s.get("AppUid").is_some() && s.get("CtId").and_then(Value::as_i64) == Some(enemy_ctid)
        })
    }

    fn find_any_snapshot_by_pid(sections: &[Value], pid: i64) -> Option<&Value> {
        sections.iter().find(|s| {
            s.get("AppUid").is_some() && s.get("PId").and_then(Value::as_i64) == Some(pid)
        })
    }

    fn find_any_snapshot_by_ctid(sections: &[Value], ctid: i64) -> Option<&Value> {
        sections.iter().find(|s| {
            s.get("AppUid").is_some() && s.get("CtId").and_then(Value::as_i64) == Some(ctid)
        })
    }

    fn count_group_players(group: &[Value]) -> usize {
        group
            .iter()
            .find_map(|s| s.get("STs").and_then(Value::as_object))
            .map(|sts| sts.keys().filter(|k| k.as_str() != "-2").count())
            .unwrap_or(0)
    }

    fn get_ots_entry_for_ctid(group: &[Value], ctid: i64) -> Option<&Value> {
        let mut buf = itoa::Buffer::new();
        let key = buf.format(ctid);
        group.iter().find_map(|s| {
            s.get("OTs")
                .and_then(Value::as_object)
                .and_then(|o| o.get(key))
        })
    }

    fn find_alliance_section_by_ct(group: &[Value], ct: i32) -> Option<&Value> {
        group.iter().find(|s| {
            s.get("AName").is_some() && s.get("CT").and_then(Value::as_i64) == Some(ct as i64)
        })
    }

    fn section_formation(section: &Value) -> Option<i32> {
        section
            .get("HFMs")
            .and_then(Value::as_i64)
            .filter(|&val| val != 0)
            .map(|val| val as i32)
    }

    fn find_formation_hint(
        sections: &[Value],
        enemy_ctid: i64,
        enemy_ct: i32,
        enemy_abbr: &str,
    ) -> Option<i32> {
        if enemy_ctid != 0
            && let Some(found) = sections.iter().find_map(|sec| {
                if sec.get("CtId").and_then(Value::as_i64) == Some(enemy_ctid) {
                    Self::section_formation(sec)
                } else {
                    None
                }
            })
        {
            return Some(found);
        }

        if enemy_ct != 0
            && let Some(found) = sections.iter().find_map(|sec| {
                if sec.get("CT").and_then(Value::as_i64).map(|ct| ct as i32) == Some(enemy_ct) {
                    Self::section_formation(sec)
                } else {
                    None
                }
            })
        {
            return Some(found);
        }

        let abbr_trimmed = enemy_abbr.trim();
        if !abbr_trimmed.is_empty()
            && let Some(found) = sections.iter().find_map(|sec| {
                if let Some(sec_abbr) = sec.get("Abbr").and_then(Value::as_str) {
                    let trimmed = sec_abbr.trim();
                    if !trimmed.is_empty() && trimmed == abbr_trimmed {
                        return Self::section_formation(sec);
                    }
                }
                None
            })
        {
            return Some(found);
        }

        None
    }

    fn find_equipment2_hint(
        sections: &[Value],
        enemy_ctid: i64,
        enemy_ct: i32,
        enemy_abbr: &str,
    ) -> Option<String> {
        if enemy_ctid != 0
            && let Some(eq2) = sections.iter().find_map(|sec| {
                if sec.get("CtId").and_then(Value::as_i64) == Some(enemy_ctid) {
                    sec.get("HEq2").and_then(Value::as_str)
                } else {
                    None
                }
            })
        {
            return Some(eq2.to_owned());
        }

        if enemy_ct != 0
            && let Some(eq2) = sections.iter().find_map(|sec| {
                if sec.get("CT").and_then(Value::as_i64).map(|ct| ct as i32) == Some(enemy_ct) {
                    sec.get("HEq2").and_then(Value::as_str)
                } else {
                    None
                }
            })
        {
            return Some(eq2.to_owned());
        }

        let abbr_trimmed = enemy_abbr.trim();
        if !abbr_trimmed.is_empty()
            && let Some(eq2) = sections.iter().find_map(|sec| {
                if let Some(sec_abbr) = sec.get("Abbr").and_then(Value::as_str) {
                    let trimmed = sec_abbr.trim();
                    if !trimmed.is_empty() && trimmed == abbr_trimmed {
                        return sec.get("HEq2").and_then(Value::as_str);
                    }
                }
                None
            })
        {
            return Some(eq2.to_owned());
        }

        None
    }

    fn select_enemy_from_ots(group: &[Value], enemy_pid: i64) -> (i64, String, i32, String) {
        let mut sources: Vec<(&str, &Value)> = Vec::new();

        for sec in group {
            if let Some(ots) = sec.get("OTs").and_then(Value::as_object) {
                sources.extend(ots.iter().map(|(k, v)| (k.as_str(), v)));
            }

            if let Some(attacks) = sec.get("Attacks").and_then(Value::as_object) {
                if let Some(ots) = attacks.get("OTs").and_then(Value::as_object) {
                    sources.extend(ots.iter().map(|(k, v)| (k.as_str(), v)));
                }

                for attack in attacks.values() {
                    if let Some(obj) = attack.as_object()
                        && let Some(ots) = obj.get("OTs").and_then(Value::as_object)
                    {
                        sources.extend(ots.iter().map(|(k, v)| (k.as_str(), v)));
                    }
                }
            }
        }

        if sources.is_empty() {
            return (0, " ".into(), 1, " ".into());
        }

        let mut candidates: Vec<(&str, &Value)> = sources
            .iter()
            .copied()
            .filter(|(_, v)| v.get("PId").and_then(Value::as_i64) == Some(enemy_pid))
            .collect();

        if candidates.is_empty() {
            candidates = sources.clone();
        }

        let mut best_k: Option<&str> = None;
        let mut best_v: Option<&Value> = None;
        let mut best_flags: (bool, bool, bool) = (false, false, false);

        for (k, v) in candidates {
            let pname = v.get("PName").and_then(Value::as_str).unwrap_or("").trim();
            let abbr = v.get("Abbr").and_then(Value::as_str).unwrap_or("").trim();

            let flags = (k != "-2", !pname.is_empty(), !abbr.is_empty());

            if best_k.is_none() {
                best_k = Some(k);
                best_v = Some(v);
                best_flags = flags;
                continue;
            }

            if flags > best_flags {
                best_k = Some(k);
                best_v = Some(v);
                best_flags = flags;
            }
        }

        if let (Some(ctid_str), Some(v)) = (best_k, best_v) {
            let ctid = ctid_str.parse::<i64>().unwrap_or(0);
            let abbr = v
                .get("Abbr")
                .and_then(Value::as_str)
                .unwrap_or(" ")
                .to_owned();
            let ct = v.get("CT").and_then(Value::as_i64).unwrap_or(1) as i32;
            let pn = v
                .get("PName")
                .and_then(Value::as_str)
                .unwrap_or(" ")
                .to_owned();
            return (ctid, abbr, ct, pn);
        }

        (0, " ".into(), 1, " ".into())
    }

    fn attack_section_get<'a>(attack_section: Option<&'a Value>, key: &str) -> Option<&'a Value> {
        attack_section.and_then(|sec| {
            sec.get(key).or_else(|| {
                sec.get("Attacks")
                    .and_then(Value::as_object)
                    .and_then(|att| att.get(key))
            })
        })
    }

    fn value_i32_nonzero(v: Option<&Value>) -> Option<i32> {
        v.and_then(Value::as_i64)
            .filter(|&x| x != 0)
            .map(|x| x as i32)
    }

    fn is_all_digits_zero_or_one(s: &str) -> bool {
        s.chars().all(|c| matches!(c, '0' | '1'))
    }

    fn compute_primary_skill_level(c_idt: &Value, enemy_snap: &Value) -> i64 {
        let primary = c_idt
            .get("HSS")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if primary >= 5 {
            return primary;
        }
        let hst = enemy_snap.get("HSt").and_then(Value::as_i64).unwrap_or(0);
        let hst2 = enemy_snap.get("HSt2").and_then(Value::as_i64).unwrap_or(0);
        if hst >= 6 || hst2 >= 6 { 5 } else { primary }
    }

    fn normalize_primary_skill_string(
        mut hss: String,
        enemy_snap: &Value,
        c_idt: &Value,
    ) -> String {
        if hss.len() == 4 && Self::is_all_digits_zero_or_one(&hss) {
            let prim5 = c_idt
                .get("HSS")
                .and_then(|o| o.get("SkillLevel"))
                .and_then(Value::as_i64)
                == Some(5);
            let hst = enemy_snap.get("HSt").and_then(Value::as_i64).unwrap_or(0);
            let hst2 = enemy_snap.get("HSt2").and_then(Value::as_i64).unwrap_or(0);
            if prim5 || hst >= 6 || hst2 >= 6 {
                hss = "5555".to_owned();
            }
        }
        hss
    }

    fn normalize_secondary_skill_string(mut hss2: String, enemy_snap: &Value) -> String {
        if hss2.len() == 4 && Self::is_all_digits_zero_or_one(&hss2) {
            let hst = enemy_snap.get("HSt").and_then(Value::as_i64).unwrap_or(0);
            let hst2 = enemy_snap.get("HSt2").and_then(Value::as_i64).unwrap_or(0);
            if hst >= 6 || hst2 >= 6 {
                hss2 = "5555".to_owned();
            }
        }
        hss2
    }

    fn find_attack_record_container(
        group: &[Value],
        attack_cid: i64,
    ) -> Option<(usize, bool, &Value, &Value)> {
        let mut buf = itoa::Buffer::new();
        let key = buf.format(attack_cid);
        for (i, sec) in group.iter().enumerate() {
            if let Some(attacks) = sec.get("Attacks")
                && let Some(obj) = attacks.get(key)
            {
                // Located within Attacks map; mark as nested.
                return Some((i, true, attacks, obj));
            }
            if let Some(obj) = sec.get(key) {
                // Located as a top-level keyed object on the section.
                return Some((i, false, sec, obj));
            }
        }
        None
    }

    fn infer_primary_commander_skills_precise(
        group: &[Value],
        attack_cid: i64,
        c_idt: &Value,
        players: usize,
    ) -> Option<String> {
        if players > 1 {
            return None;
        }
        let s1 = c_idt.get("SkillLevel").and_then(Value::as_i64)?;
        let s4 = c_idt
            .get("HSS")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(Value::as_i64)?;
        let (_idx, is_attacks, container, obj) =
            Self::find_attack_record_container(group, attack_cid)?;
        if is_attacks {
            return None;
        }

        let pair_a: Option<(i64, i64)> = obj
            .get("SkillId")
            .and_then(Value::as_i64)
            .zip(obj.get("SkillLevel").and_then(Value::as_i64));
        let pair_b: Option<(i64, i64)> = container
            .get("SkillId")
            .and_then(Value::as_i64)
            .zip(container.get("SkillLevel").and_then(Value::as_i64));
        match (pair_a, pair_b) {
            (Some(mut a), Some(mut b)) => {
                if a.0 > b.0 {
                    core::mem::swap(&mut a, &mut b);
                }
                let cid_skill_id = c_idt
                    .get("SkillId")
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                let both_mid_high = a.0 >= 1000 && b.0 >= 1000;
                if !(cid_skill_id > 0 && cid_skill_id < 1000 && both_mid_high) {
                    return None;
                }
                if (b.0 - a.0).abs() != 1 {
                    return None;
                }
                if a.1 >= 5 && b.1 >= 5 && s4 < 5 {
                    return None;
                }
                Some(format!(
                    "{}{}{}{}",
                    s1.clamp(0, 9),
                    a.1.clamp(0, 9),
                    b.1.clamp(0, 9),
                    s4.clamp(0, 9)
                ))
            }
            _ => None,
        }
    }

    fn infer_secondary_commander_skills_precise(
        group: &[Value],
        attack_cid: i64,
        players: usize,
    ) -> Option<String> {
        if players > 1 {
            return None;
        }
        let (base_idx, s4) = group.iter().enumerate().find_map(|(i, sec)| {
            (sec.get("CId").and_then(Value::as_i64) == Some(attack_cid))
                .then(|| {
                    sec.get("HSS2")
                        .and_then(|o| o.get("SkillLevel"))
                        .and_then(Value::as_i64)
                        .map(|lv| (i, lv))
                })
                .flatten()
        })?;
        let base = &group[base_idx];
        let base_id = base.get("SkillId").and_then(Value::as_i64)?;

        let mut buf: [(i64, i64); 3] = [(0, 0); 3];
        let mut got = 0usize;

        if let (Some(id), Some(lv)) = (
            base.get("SkillId").and_then(Value::as_i64),
            base.get("SkillLevel").and_then(Value::as_i64),
        ) {
            buf[got] = (id, lv);
            got += 1;
        }

        let mut j = base_idx + 1;
        while got < 3 && j < group.len() && j < base_idx + 6 {
            if let (Some(id), Some(lv)) = (
                group[j].get("SkillId").and_then(Value::as_i64),
                group[j].get("SkillLevel").and_then(Value::as_i64),
            ) && ((id / 100) == (base_id / 100) || (id - base_id).abs() <= 10)
            {
                // Collect adjacent skill ids that look like part of the same commander skill set.
                buf[got] = (id, lv);
                got += 1;
            }
            j += 1;
        }
        if got < 3 {
            return None;
        }

        buf.sort_by_key(|(id, _)| *id);
        // Require a run of three consecutive skill ids (e.g., 1000/1001/1002) before building the string.
        if !(buf[1].0 == buf[0].0 + 1 && buf[2].0 == buf[1].0 + 1) {
            return None;
        }

        let (s1, s2, s3) = (buf[0].1, buf[1].1, buf[2].1);
        Some(format!(
            "{}{}{}{}",
            s1.clamp(0, 9),
            s2.clamp(0, 9),
            s3.clamp(0, 9),
            s4.clamp(0, 9)
        ))
    }

    fn infer_primary_commander_skills(
        group: &[Value],
        attack_cid: i64,
        c_idt: &Value,
        enemy_snap: &Value,
        players: usize,
    ) -> String {
        if c_idt.get("PId").and_then(Value::as_i64) != Some(-2)
            && let Some(smart) =
                Self::infer_primary_commander_skills_precise(group, attack_cid, c_idt, players)
        {
            return Self::normalize_primary_skill_string(smart, enemy_snap, c_idt);
        }
        let primary = Self::compute_primary_skill_level(c_idt, enemy_snap);
        if primary <= 0 {
            return String::new();
        }

        // npc
        if c_idt.get("PId").and_then(Value::as_i64) == Some(-2) {
            return Self::normalize_primary_skill_string(
                format!("{}000", primary.clamp(0, 9)),
                enemy_snap,
                c_idt,
            );
        }

        let mut secondary = group
            .iter()
            .find(|s| s.get("CId").and_then(Value::as_i64) == Some(attack_cid))
            .and_then(|sec| {
                sec.get("HSS2")
                    .and_then(|o| o.get("SkillLevel"))
                    .and_then(Value::as_i64)
            });

        if secondary.is_none()
            && let Some((i, _)) = group
                .iter()
                .enumerate()
                .find(|(_, s)| s.get("CId").and_then(Value::as_i64) == Some(attack_cid))
        {
            let start = i.saturating_sub(2);
            let end = (i + 2).min(group.len().saturating_sub(1));
            // Look a few sections around the attack record for a nearby HSS2 entry.
            secondary = group[start..=end].iter().find_map(|v| {
                v.get("HSS2")
                    .and_then(|o| o.get("SkillLevel"))
                    .and_then(Value::as_i64)
            });
        }

        let tail = secondary.unwrap_or(primary);
        Self::normalize_primary_skill_string(
            format!("{0}{0}{0}{1}", primary.clamp(0, 9), tail.clamp(0, 9)),
            enemy_snap,
            c_idt,
        )
    }

    fn infer_secondary_commander_skills(group: &[Value], enemy_snap: &Value) -> String {
        let lvl = group.iter().find_map(|s| {
            s.get("HSS2")
                .and_then(|o| o.get("SkillLevel"))
                .and_then(Value::as_i64)
        });
        match lvl {
            Some(x) => Self::normalize_secondary_skill_string(
                format!("{0}{0}{0}{0}", x.clamp(0, 9)),
                enemy_snap,
            ),
            None => String::new(),
        }
    }
}

impl Resolver for ParticipantEnemyResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let sections = ctx.sections;
        let group = ctx.group;
        let (idx_opt, atk_block_opt) = find_attack_block_best_match(group, ctx.attack_id);
        let atk_block = atk_block_opt.unwrap_or(&Value::Null);
        let attack_cid = Self::parse_attack_identifier(ctx.attack_id).0.unwrap_or(0);
        let attack_container = Self::find_attack_record_container(group, attack_cid);
        let (attack_cid_opt, attack_id_str) = Self::parse_attack_identifier(ctx.attack_id);
        let attack_section = idx_opt.and_then(|idx| group.get(idx)).or_else(|| {
            // Find a nearby section that references this attack id and also carries player metadata.
            group.iter().find(|sec| {
                let idt_match = sec
                    .get("Idt")
                    .and_then(Value::as_str)
                    .map(|id| id == attack_id_str)
                    .unwrap_or(false)
                    || attack_cid_opt.is_some_and(|cid| {
                        sec.get("Idt")
                            .and_then(Value::as_str)
                            .and_then(|id| id.parse::<i64>().ok())
                            == Some(cid)
                            || sec.get("CId").and_then(Value::as_i64) == Some(cid)
                    });
                let has_metadata = sec
                    .get("PName")
                    .or_else(|| sec.get("Abbr"))
                    .or_else(|| sec.get("HId2"))
                    .is_some();
                idt_match && has_metadata
            })
        });

        let enemy_obj = get_or_insert_object_map(mail, "enemy");

        enemy_obj.insert("is_ranged_tower".into(), Value::Bool(false));

        let mut c_idt = atk_block.get("CIdt").unwrap_or_else(|| {
            group
                .iter()
                .find_map(|s| s.get("Attacks").and_then(|a| a.get("CIdt")))
                .unwrap_or(&Value::Null)
        });

        if (c_idt.get("PId").and_then(Value::as_i64).is_none() || c_idt.get("Avatar").is_none())
            && let Some(anchor) = idx_opt
        {
            let mut candidate: Option<&Value> = None;
            for d in 0..=8 {
                if anchor >= d
                    && let Some(ci) = group.get(anchor - d).and_then(|sec| {
                        sec.get("CIdt").or_else(|| {
                            sec.get("Attacks")
                                .and_then(Value::as_object)
                                .and_then(|a| a.get("CIdt"))
                        })
                    })
                {
                    candidate = Some(ci);
                    break;
                }
                if let Some(ci) = group.get(anchor + d).and_then(|sec| {
                    sec.get("CIdt").or_else(|| {
                        sec.get("Attacks")
                            .and_then(Value::as_object)
                            .and_then(|a| a.get("CIdt"))
                    })
                }) {
                    candidate = Some(ci);
                    break;
                }
            }

            if let Some(ci) = candidate {
                c_idt = ci;
            }
        }

        let enemy_pid = c_idt.get("PId").and_then(Value::as_i64).unwrap_or(-2);
        let (enemy_ctid, enemy_abbr, enemy_ct, enemy_pname) =
            Self::select_enemy_from_ots(group, enemy_pid);

        let mut enemy_snap =
            Self::find_enemy_snapshot_by_identifier_strict(group, attack_id_str, attack_cid_opt)
                .or_else(|| Self::find_enemy_snapshot_by_ctid(group, enemy_ctid))
                .or_else(|| {
                    Self::find_enemy_snapshot_by_identifier_loose(
                        group,
                        attack_id_str,
                        attack_cid_opt,
                    )
                });
        if enemy_snap.is_none() {
            // If the structured snapshot is missing, rely on the broader attack section for metadata.
            enemy_snap = attack_section;
        }

        let mut is_ranged_tower = enemy_snap
            .and_then(|snap| snap.get("IsRangeTower").and_then(Value::as_bool))
            .unwrap_or(false);
        if !is_ranged_tower {
            if let Some(flag) = c_idt.get("IsRangeTower").and_then(Value::as_bool) {
                is_ranged_tower = flag;
            } else if let Some(flag) = atk_block.get("IsRangeTower").and_then(Value::as_bool) {
                is_ranged_tower = flag;
            }
        }
        if !is_ranged_tower {
            let matches_attack = |sec: &Value| {
                let by_str = sec
                    .get("Idt")
                    .and_then(Value::as_str)
                    .map(|id| id == attack_id_str)
                    .unwrap_or(false);
                let by_num = attack_cid_opt.is_some_and(|cid| {
                    sec.get("Idt")
                        .and_then(Value::as_str)
                        .and_then(|id| id.parse::<i64>().ok())
                        == Some(cid)
                        || sec.get("CId").and_then(Value::as_i64) == Some(cid)
                });
                let by_ctid = sec
                    .get("CtId")
                    .and_then(Value::as_i64)
                    .map(|v| v == enemy_ctid)
                    .unwrap_or(false);
                by_str || by_num || by_ctid
            };

            if group.iter().any(|s| {
                s.get("IsRangeTower")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    && matches_attack(s)
            }) || sections.iter().any(|s| {
                s.get("IsRangeTower")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    && matches_attack(s)
            }) {
                is_ranged_tower = true;
            }
        }
        enemy_obj.insert("is_ranged_tower".into(), Value::Bool(is_ranged_tower));

        let resolved_pid = c_idt.get("PId").and_then(Value::as_i64).or_else(|| {
            Self::get_ots_entry_for_ctid(group, enemy_ctid)
                .and_then(|o| o.get("PId"))
                .and_then(Value::as_i64)
        });

        if c_idt.get("PId").and_then(Value::as_i64).is_none() || c_idt.get("Avatar").is_none() {
            if let Some(anchor) = idx_opt {
                let mut candidate: Option<&Value> = None;
                for d in 0..=8 {
                    if candidate.is_some() {
                        break;
                    }
                    // Expand search around the anchor to reuse a CIdt that actually carries pid/avatar data.
                    if anchor >= d
                        && let Some(sec) = group.get(anchor - d)
                        && let Some(ci) = sec
                            .get("CIdt")
                            .or_else(|| sec.get("Attacks").and_then(|a| a.get("CIdt")))
                    {
                        let pid_match = ci.get("PId").and_then(Value::as_i64).unwrap_or(-3);
                        if resolved_pid.is_some() && Some(pid_match) == resolved_pid {
                            candidate = Some(ci);
                            break;
                        }
                        if candidate.is_none() {
                            candidate = Some(ci);
                        }
                    }
                    if let Some(sec) = group.get(anchor + d)
                        && let Some(ci) = sec
                            .get("CIdt")
                            .or_else(|| sec.get("Attacks").and_then(|a| a.get("CIdt")))
                    {
                        let pid_match = ci.get("PId").and_then(Value::as_i64).unwrap_or(-3);
                        if resolved_pid.is_some() && Some(pid_match) == resolved_pid {
                            candidate = Some(ci);
                            break;
                        }
                        if candidate.is_none() {
                            candidate = Some(ci);
                        }
                    }
                }
                if let Some(ci) = candidate {
                    c_idt = ci;
                }
            }
            if (c_idt.get("PId").and_then(Value::as_i64).is_none() || c_idt.get("Avatar").is_none())
                && resolved_pid.is_some()
                && let Some(ci) = group
                    .iter()
                    .find_map(|s| s.get("CIdt"))
                    .filter(|ci| ci.get("PId").and_then(Value::as_i64) == resolved_pid)
            {
                c_idt = ci;
            }
        }

        let players = Self::count_group_players(group);

        // player id
        let pid = resolved_pid.or_else(|| c_idt.get("PId").and_then(Value::as_i64));
        map_put_i64(enemy_obj, "player_id", pid);

        // player name
        let attack_section_name = Self::attack_section_get(attack_section, "PName")
            .and_then(Value::as_str)
            .map(|s| s.to_owned());

        if let Some(name) = (!enemy_pname.trim().is_empty())
            .then_some(enemy_pname.clone())
            .or_else(|| {
                enemy_snap
                    .and_then(|s| s.get("PName").and_then(Value::as_str).map(|s| s.to_owned()))
            })
            .or_else(|| attack_section_name.clone())
        {
            enemy_obj.insert("player_name".into(), Value::String(name));
        }

        // alliance
        if !enemy_abbr.trim().is_empty() {
            enemy_obj.insert("alliance_tag".into(), Value::String(enemy_abbr.clone()));
        } else if let Some(snap) = enemy_snap
            && let Some(abbr2) = snap
                .get("Abbr")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        {
            enemy_obj.insert("alliance_tag".into(), Value::String(abbr2.to_owned()));
        } else if let Some(abbr3) = Self::attack_section_get(attack_section, "Abbr")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            enemy_obj.insert("alliance_tag".into(), Value::String(abbr3.to_owned()));
        }

        // avatar url
        if enemy_obj.get("avatar_url").is_none() {
            if let Some(url) = extract_avatar_url(c_idt.get("Avatar")) {
                enemy_obj.insert("avatar_url".into(), Value::String(url));
            } else if let Some(url) = enemy_snap.and_then(|s| extract_avatar_url(s.get("Avatar"))) {
                enemy_obj.insert("avatar_url".into(), Value::String(url));
            } else if let Some(pid) = pid
                && let Some(full_snap) = Self::find_any_snapshot_by_pid(sections, pid)
                && let Some(url) = extract_avatar_url(full_snap.get("Avatar"))
            {
                enemy_obj.insert("avatar_url".into(), Value::String(url));
            } else if let Some(anchor) = idx_opt {
                let mut found: Option<String> = None;
                for d in 1..=6 {
                    if found.is_some() {
                        break;
                    }
                    if anchor >= d
                        && let Some(url) = extract_avatar_url(group[anchor - d].get("Avatar"))
                    {
                        found = Some(url);
                        break;
                    }
                    if let Some(sec) = group.get(anchor + d)
                        && let Some(url) = extract_avatar_url(sec.get("Avatar"))
                    {
                        found = Some(url);
                        break;
                    }
                }
                if let Some(url) = found {
                    enemy_obj.insert("avatar_url".into(), Value::String(url));
                }
            }
        }

        // avatar frame
        if enemy_obj.get("frame_url").is_none() {
            let frame_url = extract_avatar_frame_url(c_idt.get("Avatar"))
                .or_else(|| enemy_snap.and_then(|s| extract_avatar_frame_url(s.get("Avatar"))))
                .or_else(|| {
                    pid.and_then(|pid| {
                        Self::find_any_snapshot_by_pid(sections, pid)
                            .and_then(|s| extract_avatar_frame_url(s.get("Avatar")))
                    })
                })
                .unwrap_or_default();
            enemy_obj.insert("frame_url".into(), Value::String(frame_url));
        }

        // app uid
        let mut app_uid = enemy_snap
            .and_then(|snap| extract_app_uid(snap.get("AppUid")))
            .or_else(|| extract_app_uid(c_idt.get("AppUid")))
            .or_else(|| extract_app_uid(atk_block.get("AppUid")))
            .or_else(|| {
                Self::attack_section_get(attack_section, "AppUid")
                    .and_then(|v| extract_app_uid(Some(v)))
            });
        if app_uid.is_none()
            && let Some(pid) = pid
        {
            app_uid = Self::find_any_snapshot_by_pid(group, pid)
                .and_then(|snap| extract_app_uid(snap.get("AppUid")))
                .or_else(|| {
                    Self::find_any_snapshot_by_pid(sections, pid)
                        .and_then(|snap| extract_app_uid(snap.get("AppUid")))
                });
        }
        if app_uid.is_none() && enemy_ctid != 0 {
            app_uid = Self::find_enemy_snapshot_by_ctid(group, enemy_ctid)
                .and_then(|snap| extract_app_uid(snap.get("AppUid")))
                .or_else(|| {
                    Self::find_any_snapshot_by_ctid(sections, enemy_ctid)
                        .and_then(|snap| extract_app_uid(snap.get("AppUid")))
                });
        }
        if app_uid.is_none() {
            app_uid = extract_app_uid_from_avatar_url(enemy_obj.get("avatar_url"));
        }
        if let Some(uid) = app_uid {
            enemy_obj.insert("app_uid".into(), Value::String(uid));
        }

        // castle pos
        if let Some(snap) = enemy_snap
            && let Some(castle) = snap.get("CastlePos")
        {
            map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
            map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
        }
        if (enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none())
            && let Some(castle) = atk_block.get("CastlePos")
        {
            map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
            map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
        }
        if (enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none())
            && let Some((_i, is_attacks, container, _obj)) = attack_container
            && is_attacks
            && let Some(castle) = container.get("CastlePos")
        {
            map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
            map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
        }
        if (enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none())
            && let Some(i) = idx_opt
        {
            if let Some(castle) = group.get(i).and_then(|sec| sec.get("CastlePos")) {
                map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
                map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
            }
            if enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none() {
                for d in 1..=8 {
                    if i >= d
                        && let Some(castle) = group.get(i - d).and_then(|sec| sec.get("CastlePos"))
                    {
                        map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
                        map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
                        break;
                    }
                    if let Some(castle) = group.get(i + d).and_then(|sec| sec.get("CastlePos")) {
                        map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
                        map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
                        break;
                    }
                }
            }
        }

        if enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none() {
            if let Some(pid) = pid
                && let Some(snap) =
                    Self::find_any_snapshot_by_pid(sections, pid).and_then(|s| s.get("CastlePos"))
            {
                map_put_f64(enemy_obj, "castle_x", parse_f64(snap.get("X")));
                map_put_f64(enemy_obj, "castle_y", parse_f64(snap.get("Y")));
            }
            if (enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none())
                && enemy_ctid != 0
                && let Some(snap) = Self::find_any_snapshot_by_ctid(sections, enemy_ctid)
                    .and_then(|s| s.get("CastlePos"))
            {
                map_put_f64(enemy_obj, "castle_x", parse_f64(snap.get("X")));
                map_put_f64(enemy_obj, "castle_y", parse_f64(snap.get("Y")));
            }
            if enemy_obj.get("castle_x").is_none() || enemy_obj.get("castle_y").is_none() {
                let abbr_trimmed = enemy_abbr.trim();
                let abbr_matches = |sec: &Value| -> bool {
                    if abbr_trimmed.is_empty() {
                        true
                    } else {
                        sec.get("Abbr")
                            .and_then(Value::as_str)
                            .map(|abbr| abbr.trim())
                            == Some(abbr_trimmed)
                    }
                };

                let mut castle_from_alliance = if !abbr_trimmed.is_empty() {
                    group
                        .iter()
                        .find_map(|sec| sec.get("CastlePos").filter(|_| abbr_matches(sec)))
                        .or_else(|| {
                            sections
                                .iter()
                                .find_map(|sec| sec.get("CastlePos").filter(|_| abbr_matches(sec)))
                        })
                } else {
                    None
                };

                if castle_from_alliance.is_none() && enemy_ct != 0 {
                    castle_from_alliance = Self::find_alliance_section_by_ct(group, enemy_ct)
                        .filter(|sec| abbr_matches(sec))
                        .and_then(|sec| sec.get("CastlePos"))
                        .or_else(|| {
                            Self::find_alliance_section_by_ct(sections, enemy_ct)
                                .filter(|sec| abbr_matches(sec))
                                .and_then(|sec| sec.get("CastlePos"))
                        });
                }

                if let Some(castle) = castle_from_alliance {
                    map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
                    map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
                }
            }
        }

        // rally
        let ctk_lower_has_rally = enemy_snap
            .and_then(|s| s.get("CTK").and_then(Value::as_str))
            .map(|ctk| ctk.to_ascii_lowercase().contains("rally"))
            .unwrap_or(false);
        let is_rally = enemy_snap
            .and_then(|s| s.get("IsRally").and_then(Value::as_bool))
            .unwrap_or(false)
            || ctk_lower_has_rally;
        if is_rally {
            enemy_obj.insert("is_rally".into(), Value::from(1));
        }

        // alliance building type
        let ab_t = enemy_snap
            .and_then(|s| s.get("AbT"))
            .or_else(|| c_idt.get("AbT"))
            .or_else(|| atk_block.get("AbT"))
            .or_else(|| Self::attack_section_get(attack_section, "AbT"))
            .and_then(Value::as_i64)
            .map(|v| v as i32);
        map_put_i32(enemy_obj, "alliance_building", ab_t);

        // npc
        map_put_i32(
            enemy_obj,
            "npc_type",
            atk_block
                .get("NpcType")
                .or_else(|| Self::attack_section_get(attack_section, "NpcType"))
                .and_then(Value::as_i64)
                .map(|x| x as i32),
        );
        map_put_i32(
            enemy_obj,
            "npc_btype",
            atk_block
                .get("NpcBType")
                .or_else(|| Self::attack_section_get(attack_section, "NpcBType"))
                .and_then(Value::as_i64)
                .map(|x| x as i32),
        );

        // commanders
        let hid = Self::value_i32_nonzero(c_idt.get("HId"))
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| Self::value_i32_nonzero(o.get("HId")))
            })
            .or_else(|| Self::value_i32_nonzero(Self::attack_section_get(attack_section, "HId")));
        let hlv = Self::value_i32_nonzero(c_idt.get("HLv"))
            .or_else(|| {
                Self::find_alliance_section_by_ct(group, enemy_ct)
                    .and_then(|a| Self::value_i32_nonzero(a.get("HLv")))
            })
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| Self::value_i32_nonzero(o.get("HLv")))
            })
            .or_else(|| Self::value_i32_nonzero(atk_block.get("HLv")))
            .or_else(|| Self::value_i32_nonzero(Self::attack_section_get(attack_section, "HLv")));

        if hid.is_some() || hlv.is_some() {
            let mut cmd = Map::new();
            map_put_i32(&mut cmd, "id", hid);
            map_put_i32(&mut cmd, "level", hlv);
            let hss = Self::infer_primary_commander_skills(
                group,
                attack_cid,
                c_idt,
                enemy_snap.unwrap_or(&Value::Null),
                players,
            );
            if !hss.is_empty() {
                cmd.insert("skills".into(), Value::String(hss));
            }
            enemy_obj.insert("primary_commander".into(), Value::Object(cmd));
        }

        let hid2 = Self::value_i32_nonzero(c_idt.get("HId2"))
            .or_else(|| {
                group
                    .iter()
                    .find(|s| s.get("AppUid").is_some())
                    .and_then(|snap| Self::value_i32_nonzero(snap.get("HId2")))
            })
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| Self::value_i32_nonzero(o.get("HId2")))
            })
            .or_else(|| Self::value_i32_nonzero(Self::attack_section_get(attack_section, "HId2")))
            .or_else(|| Self::value_i32_nonzero(atk_block.get("HId2")));
        let hlv2 = Self::value_i32_nonzero(c_idt.get("HLv2"))
            .or_else(|| {
                Self::find_alliance_section_by_ct(group, enemy_ct)
                    .and_then(|a| Self::value_i32_nonzero(a.get("HLv2")))
            })
            .or_else(|| {
                group
                    .iter()
                    .find(|s| s.get("AppUid").is_some())
                    .and_then(|snap| Self::value_i32_nonzero(snap.get("HLv2")))
            })
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| Self::value_i32_nonzero(o.get("HLv2")))
            })
            .or_else(|| Self::value_i32_nonzero(atk_block.get("HLv2")))
            .or_else(|| Self::value_i32_nonzero(Self::attack_section_get(attack_section, "HLv2")));

        if hid2.is_some() || hlv2.is_some() {
            let mut cmd2 = Map::new();
            map_put_i32(&mut cmd2, "id", hid2);
            map_put_i32(&mut cmd2, "level", hlv2);
            let hss2 = if enemy_snap
                .and_then(|s| s.get("HSt2").and_then(Value::as_i64))
                .unwrap_or(0)
                >= 6
            {
                Self::infer_secondary_commander_skills(group, enemy_snap.unwrap_or(&Value::Null))
            } else {
                Self::infer_secondary_commander_skills_precise(group, attack_cid, players)
                    .unwrap_or_else(|| {
                        Self::infer_secondary_commander_skills(
                            group,
                            enemy_snap.unwrap_or(&Value::Null),
                        )
                    })
            };
            if !hss2.is_empty() {
                cmd2.insert("skills".into(), Value::String(hss2));
            }
            enemy_obj.insert("secondary_commander".into(), Value::Object(cmd2));
        }

        // equipment
        if let Some(snap) = enemy_snap {
            map_put_str(
                enemy_obj,
                "equipment",
                snap.get("HEq").and_then(Value::as_str),
            );
            map_put_str(
                enemy_obj,
                "equipment_2",
                snap.get("HEq2").and_then(Value::as_str),
            );
        }
        if enemy_obj.get("equipment").is_none() {
            map_put_str(
                enemy_obj,
                "equipment",
                atk_block.get("HEq").and_then(Value::as_str),
            );
        }
        if enemy_obj.get("equipment_2").is_none() {
            map_put_str(
                enemy_obj,
                "equipment_2",
                atk_block.get("HEq2").and_then(Value::as_str),
            );
        }
        if enemy_obj.get("equipment").is_none() {
            map_put_str(
                enemy_obj,
                "equipment",
                Self::attack_section_get(attack_section, "HEq").and_then(Value::as_str),
            );
        }
        if enemy_obj.get("equipment_2").is_none() {
            map_put_str(
                enemy_obj,
                "equipment_2",
                Self::attack_section_get(attack_section, "HEq2").and_then(Value::as_str),
            );
        }
        if enemy_obj.get("equipment_2").is_none() {
            let eq2_hint = Self::find_equipment2_hint(group, enemy_ctid, enemy_ct, &enemy_abbr);
            if eq2_hint.is_none() {
                let global_hint =
                    Self::find_equipment2_hint(sections, enemy_ctid, enemy_ct, &enemy_abbr);
                map_put_str(enemy_obj, "equipment_2", global_hint.as_deref());
            } else {
                map_put_str(enemy_obj, "equipment_2", eq2_hint.as_deref());
            }
        }

        // formation / buffs / inscriptions
        if let Some(snap) = enemy_snap {
            map_put_i32(
                enemy_obj,
                "formation",
                snap.get("HFMs").and_then(Value::as_i64).map(|x| x as i32),
            );

            let buffs = collect_buffs_from_hwbs(snap.get("HWBs"));
            if !buffs.is_empty() {
                enemy_obj.insert("armament_buffs".into(), Value::String(buffs));
            }

            let insc = collect_affix_from_hwbs(snap.get("HWBs"));
            if !insc.is_empty() {
                enemy_obj.insert("inscriptions".into(), Value::String(insc));
            }
        }
        if enemy_obj.get("formation").is_none() {
            map_put_i32(
                enemy_obj,
                "formation",
                Self::attack_section_get(attack_section, "HFMs")
                    .and_then(Value::as_i64)
                    .map(|x| x as i32),
            );
        }
        if enemy_obj.get("formation").is_none() {
            let mut formation_hint =
                Self::find_formation_hint(group, enemy_ctid, enemy_ct, &enemy_abbr);
            if formation_hint.is_none() {
                formation_hint =
                    Self::find_formation_hint(sections, enemy_ctid, enemy_ct, &enemy_abbr);
            }
            map_put_i32(enemy_obj, "formation", formation_hint);
        }
        if (enemy_obj.get("armament_buffs").is_none() || enemy_obj.get("inscriptions").is_none())
            && let Some(hwbs) = Self::attack_section_get(attack_section, "HWBs")
        {
            let buffs = collect_buffs_from_hwbs(Some(hwbs));
            if !buffs.is_empty() {
                enemy_obj.insert("armament_buffs".into(), Value::String(buffs));
            }
            let insc = collect_affix_from_hwbs(Some(hwbs));
            if !insc.is_empty() {
                enemy_obj.insert("inscriptions".into(), Value::String(insc));
            }
        }

        Ok(())
    }
}
