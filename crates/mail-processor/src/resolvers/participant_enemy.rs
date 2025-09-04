use crate::{
    helpers::{
        collect_affix_from_hwbs, collect_buffs_from_hwbs, find_attack_block_best_match,
        get_or_insert_object, map_put_f64, map_put_i32, map_put_i64, map_put_str, parse_f64,
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

    fn count_group_players(group: &[Value]) -> usize {
        group
            .iter()
            .find_map(|s| s.get("STs").and_then(Value::as_object))
            .map(|sts| sts.keys().filter(|k| k.as_str() != "-2").count())
            .unwrap_or(0)
    }

    fn get_ots_entry_for_ctid(group: &[Value], ctid: i64) -> Option<&Value> {
        let key = ctid.to_string();
        group.iter().find_map(|s| {
            s.get("OTs")
                .and_then(Value::as_object)
                .and_then(|o| o.get(&key))
        })
    }

    fn find_alliance_section_by_ct(group: &[Value], ct: i32) -> Option<&Value> {
        group.iter().find(|s| {
            s.get("AName").is_some() && s.get("CT").and_then(Value::as_i64) == Some(ct as i64)
        })
    }

    fn select_enemy_from_ots(group: &[Value], enemy_pid: i64) -> (i64, String, i32, String) {
        if let Some(ots) = group
            .iter()
            .find_map(|s| s.get("OTs").and_then(Value::as_object))
        {
            let mut candidates: Vec<(&str, &Value)> = ots
                .iter()
                .filter(|(_k, v)| v.get("PId").and_then(Value::as_i64) == Some(enemy_pid))
                .map(|(k, v)| (k.as_str(), v))
                .collect();
            if candidates.is_empty() {
                candidates.extend(ots.iter().map(|(k, v)| (k.as_str(), v)));
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
        }
        (0, " ".into(), 1, " ".into())
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
                buf[got] = (id, lv);
                got += 1;
            }
            j += 1;
        }
        if got < 3 {
            return None;
        }

        buf.sort_by_key(|(id, _)| *id);
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

        let enemy_obj = match get_or_insert_object(mail, "enemy") {
            Value::Object(m) => m,
            _ => unreachable!("enemy must be an object"),
        };

        let c_idt = atk_block.get("CIdt").unwrap_or(&Value::Null);

        let enemy_pid = c_idt.get("PId").and_then(Value::as_i64).unwrap_or(-2);
        let (enemy_ctid, enemy_abbr, enemy_ct, enemy_pname) =
            Self::select_enemy_from_ots(group, enemy_pid);

        let enemy_snap =
            Self::find_enemy_snapshot_by_identifier_strict(group, attack_id_str, attack_cid_opt)
                .or_else(|| Self::find_enemy_snapshot_by_ctid(group, enemy_ctid))
                .or_else(|| {
                    Self::find_enemy_snapshot_by_identifier_loose(
                        group,
                        attack_id_str,
                        attack_cid_opt,
                    )
                });

        let players = Self::count_group_players(group);

        // player id
        let pid = c_idt.get("PId").and_then(Value::as_i64).or_else(|| {
            Self::get_ots_entry_for_ctid(group, enemy_ctid)
                .and_then(|o| o.get("PId"))
                .and_then(Value::as_i64)
        });
        map_put_i64(enemy_obj, "player_id", pid);

        // player name
        if let Some(name) = (!enemy_pname.trim().is_empty())
            .then_some(enemy_pname.clone())
            .or_else(|| {
                enemy_snap
                    .and_then(|s| s.get("PName").and_then(Value::as_str).map(|s| s.to_owned()))
            })
        {
            enemy_obj.insert("player_name".into(), Value::String(name));
        }

        // alliance
        if !enemy_abbr.trim().is_empty() {
            enemy_obj.insert("alliance_tag".into(), Value::String(enemy_abbr));
        } else if let Some(snap) = enemy_snap
            && let Some(abbr2) = snap
                .get("Abbr")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        {
            enemy_obj.insert("alliance_tag".into(), Value::String(abbr2.to_owned()));
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
            && let Some(sec) = group.get(i)
            && let Some(castle) = sec.get("CastlePos")
        {
            map_put_f64(enemy_obj, "castle_x", parse_f64(castle.get("X")));
            map_put_f64(enemy_obj, "castle_y", parse_f64(castle.get("Y")));
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

        // tracking key
        if let Some(ctk) = enemy_snap
            .and_then(|s| s.get("CTK").and_then(Value::as_str))
            .filter(|s| !s.is_empty())
        {
            enemy_obj.insert("tracking_key".into(), Value::String(ctk.to_owned()));
        } else if let Some(ctk2) = atk_block
            .get("CTK")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            enemy_obj.insert("tracking_key".into(), Value::String(ctk2.to_owned()));
        }

        // npc
        map_put_i32(
            enemy_obj,
            "npc_type",
            atk_block
                .get("NpcType")
                .and_then(Value::as_i64)
                .map(|x| x as i32),
        );
        map_put_i32(
            enemy_obj,
            "npc_btype",
            atk_block
                .get("NpcBType")
                .and_then(Value::as_i64)
                .map(|x| x as i32),
        );

        // commanders
        let hid = c_idt
            .get("HId")
            .and_then(Value::as_i64)
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| o.get("HId"))
                    .and_then(Value::as_i64)
            })
            .map(|x| x as i32);
        let hlv = c_idt
            .get("HLv")
            .and_then(Value::as_i64)
            .or_else(|| {
                Self::find_alliance_section_by_ct(group, enemy_ct)
                    .and_then(|a| a.get("HLv"))
                    .and_then(Value::as_i64)
            })
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| o.get("HLv"))
                    .and_then(Value::as_i64)
            })
            .or_else(|| atk_block.get("HLv").and_then(Value::as_i64))
            .map(|x| x as i32);

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

        let hid2 = c_idt
            .get("HId2")
            .and_then(Value::as_i64)
            .or_else(|| {
                group
                    .iter()
                    .find(|s| s.get("AppUid").is_some())
                    .and_then(|snap| snap.get("HId2"))
                    .and_then(Value::as_i64)
            })
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| o.get("HId2"))
                    .and_then(Value::as_i64)
            })
            .map(|x| x as i32);
        let hlv2 = c_idt
            .get("HLv2")
            .and_then(Value::as_i64)
            .or_else(|| {
                Self::find_alliance_section_by_ct(group, enemy_ct)
                    .and_then(|a| a.get("HLv2"))
                    .and_then(Value::as_i64)
            })
            .or_else(|| {
                group
                    .iter()
                    .find(|s| s.get("AppUid").is_some())
                    .and_then(|snap| snap.get("HLv2"))
                    .and_then(Value::as_i64)
            })
            .or_else(|| {
                Self::get_ots_entry_for_ctid(group, enemy_ctid)
                    .and_then(|o| o.get("HLv2"))
                    .and_then(Value::as_i64)
            })
            .map(|x| x as i32);

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

        // kingdom
        if let Some(snap) = enemy_snap {
            map_put_i32(
                enemy_obj,
                "kingdom_id",
                snap.get("COSId")
                    .and_then(Value::as_i64)
                    .filter(|&x| x != 0)
                    .map(|x| x as i32),
            );
        }
        if enemy_obj.get("kingdom_id").is_none() {
            map_put_i32(
                enemy_obj,
                "kingdom_id",
                c_idt
                    .get("COSId")
                    .and_then(Value::as_i64)
                    .filter(|&x| x != 0)
                    .map(|x| x as i32),
            );
        }
        if enemy_obj.get("kingdom_id").is_none() {
            let k3 = sections
                .iter()
                .find_map(|s| s.get("GsId").and_then(Value::as_i64))
                .or_else(|| {
                    sections
                        .first()
                        .and_then(|s| s.get("serverId").and_then(Value::as_i64))
                });
            map_put_i32(enemy_obj, "kingdom_id", k3.map(|x| x as i32));
        }

        // equipment
        if let Some(snap) = enemy_snap {
            map_put_str(
                enemy_obj,
                "equipment",
                snap.get("HEq").and_then(Value::as_str),
            );
        }
        if enemy_obj.get("equipment").is_none() {
            map_put_str(
                enemy_obj,
                "equipment",
                atk_block.get("HEq").and_then(Value::as_str),
            );
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

        Ok(())
    }
}
