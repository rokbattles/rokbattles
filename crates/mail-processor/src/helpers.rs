use serde_json::{Value, json};

pub fn get_or_insert_object<'a>(obj: &'a mut Value, key: &str) -> &'a mut Value {
    let map = obj.as_object_mut().expect("mail root must be an object");
    if !map.get(key).map(|v| v.is_object()).unwrap_or(false) {
        map.insert(key.to_string(), json!({}));
    }
    map.get_mut(key).unwrap()
}

pub fn get_i64_alt(v: &Value, k1: &str, k2: &str) -> Option<i64> {
    v.get(k1)
        .and_then(|x| x.as_i64())
        .or_else(|| v.get(k2).and_then(|x| x.as_i64()))
}

pub fn pick_f64(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

pub fn join_buffs(hwbs: Option<&Value>) -> String {
    let mut buffs: Vec<String> = Vec::new();
    if let Some(obj) = hwbs.and_then(|v| v.as_object()) {
        for (_k, v) in obj {
            if let Some(b) = v.get("Buffs").and_then(|x| x.as_str()) {
                buffs.extend(
                    b.split(';')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                );
            }
        }
    }
    buffs.sort();
    buffs.join(";")
}

pub fn join_affix(hwbs: Option<&Value>) -> String {
    let mut aff: Vec<String> = Vec::new();
    if let Some(obj) = hwbs.and_then(|v| v.as_object()) {
        for (_k, v) in obj {
            if let Some(a) = v.get("Affix").and_then(|x| x.as_str()) {
                aff.extend(
                    a.split(';')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                );
            }
        }
    }
    aff.sort_by(|a, b| {
        let (ia, ib) = (a.parse::<i32>().ok(), b.parse::<i32>().ok());
        match (ia, ib) {
            (Some(x), Some(y)) => x.cmp(&y),
            _ => a.cmp(b),
        }
    });
    aff.join(";")
}

pub fn find_self_snapshot(sections: &[Value]) -> Value {
    for s in sections {
        if s.get("AppUid").is_some() && s.get("CtId").and_then(|v| v.as_i64()).unwrap_or(-1) == 0 {
            return s.clone();
        }
    }
    for s in sections {
        if s.get("AppUid").is_some() {
            return s.clone();
        }
    }
    Value::Null
}

pub fn find_self_body(sections: &[Value]) -> Value {
    for s in sections {
        if s.pointer("/body/content/SelfChar").is_some() || s.pointer("/content/SelfChar").is_some()
        {
            return s
                .pointer("/body/content")
                .cloned()
                .or_else(|| s.get("content").cloned())
                .unwrap_or(Value::Null);
        }
    }
    Value::Null
}

pub fn find_enemy_snapshot_for_attack(group: &[Value], attack_id: &str) -> Option<Value> {
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

pub fn find_enemy_snapshot_by_ctid(group: &[Value], enemy_ctid: i64) -> Option<Value> {
    for s in group {
        if s.get("AppUid").is_some()
            && s.get("CtId").and_then(|v| v.as_i64()).unwrap_or(-1) == enemy_ctid
        {
            return Some(s.clone());
        }
    }
    None
}

pub fn find_best_attack_block(group: &[Value], attack_id: &str) -> (Option<usize>, Value) {
    let mut idx: Option<usize> = None;
    let mut best: Option<Value> = None;
    let mut best_has_hss = false;
    for (gi, s) in group.iter().enumerate() {
        if let Some(b) = s
            .get(attack_id)
            .or_else(|| s.pointer(&format!("/Attacks/{}", attack_id)))
        {
            let has_hss = b.get("CIdt").and_then(|c| c.get("HSS")).is_some();
            if best.is_none() || (!best_has_hss && has_hss) {
                idx = Some(gi);
                best = Some(b.clone());
                best_has_hss = has_hss;
                if best_has_hss {
                    break;
                }
            }
        }
    }
    if best.is_none() {
        for (gi, s) in group.iter().enumerate() {
            let idt_match = s
                .get("Idt")
                .and_then(|v| v.as_str())
                .map(|x| x == attack_id)
                .unwrap_or(false)
                || s.get("Idt")
                    .and_then(|v| v.as_i64())
                    .map(|x| x.to_string() == attack_id)
                    .unwrap_or(false);
            if idt_match
                && (s.get("HSS").is_some() || s.get("HId").is_some() || s.get("HId2").is_some())
            {
                idx = Some(gi);
                best = Some(Value::Object(serde_json::Map::new()));
                break;
            }
        }
    }
    (idx, best.unwrap_or(Value::Null))
}

pub fn pick_enemy_from_ots(group: &[Value], enemy_pid: i64) -> (i64, String, i32, String) {
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

pub fn find_group_aname_ct(group: &[Value], ct: i32) -> Option<&Value> {
    group.iter().find(|s| {
        s.get("AName").is_some() && s.get("CT").and_then(|v| v.as_i64()).unwrap_or(-1) as i32 == ct
    })
}

pub fn group_ots_entry(group: &[Value], ctid: i64) -> Option<&Value> {
    group.iter().find_map(|s| {
        s.get("OTs")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get(&ctid.to_string()))
    })
}

pub fn parse_hids_from_ctk(ctk: Option<&str>) -> (Option<i64>, Option<i64>) {
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

pub fn pick_hlv2(sections: &[Value], self_snap: &Value) -> i32 {
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

pub fn pick_hss2_fourdigits(sections: &[Value]) -> String {
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

pub fn compose_hss_mailwide(sections: &[Value], self_body: &Value) -> String {
    let mut digits: [Option<i64>; 4] = [None, None, None, None];

    let self_idx = sections.iter().position(|s| {
        s.pointer("/body/content/SelfChar").is_some() || s.pointer("/content/SelfChar").is_some()
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
    if hss.len() == 4 && digits_all_leq_one(&hss) {
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
    if hss2.len() == 4 && digits_all_leq_one(&hss2) {
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

fn compose_enemy_hss_precise(group: &[Value], attack_cid: i64, c_idt: &Value) -> Option<String> {
    if group_players_count(group) > 1 {
        return None;
    }

    let s1 = c_idt.get("SkillLevel").and_then(|v| v.as_i64())?;
    let s4 = c_idt
        .get("HSS")
        .and_then(|o| o.get("SkillLevel"))
        .and_then(|v| v.as_i64())?;

    let (_idx, is_attacks, container, obj) = find_attack_container(group, attack_cid)?;

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

pub fn compose_enemy_hss2_precise(group: &[Value], attack_cid: i64) -> Option<String> {
    if group_players_count(group) > 1 {
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

pub fn compose_enemy_hss(
    group: &[Value],
    attack_cid: i64,
    c_idt: &Value,
    enemy_snap: &Value,
) -> String {
    if c_idt.get("PId").and_then(|v| v.as_i64()) != Some(-2)
        && let Some(smart) = compose_enemy_hss_precise(group, attack_cid, c_idt)
    {
        return normalize_hss(smart, enemy_snap, c_idt);
    }

    let primary = compute_primary_level(c_idt, enemy_snap);
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
        return normalize_hss(s, enemy_snap, c_idt);
    }

    let tail = secondary.unwrap_or(primary);
    let s = format!(
        "{}{}{}{}",
        primary.clamp(0, 9),
        primary.clamp(0, 9),
        primary.clamp(0, 9),
        tail.clamp(0, 9)
    );

    normalize_hss(s, enemy_snap, c_idt)
}

pub fn compose_enemy_hss2(group: &[Value], enemy_snap: &Value) -> String {
    let lvl = group.iter().find_map(|s| {
        s.get("HSS2")
            .and_then(|o| o.get("SkillLevel"))
            .and_then(|x| x.as_i64())
    });
    match lvl {
        Some(x) => normalize_hss2(format!("{0}{0}{0}{0}", x), enemy_snap),
        None => String::new(),
    }
}

pub fn pick_self_abbr_ct_form(sections: &[Value]) -> (String, i32, i32) {
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
