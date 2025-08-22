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

pub fn pick_i64(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) if s.chars().all(|c| c == '-' || c.is_ascii_digit()) => {
            s.parse().ok()
        }
        _ => None,
    }
}

pub fn pick_f64(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

pub fn epoch_seconds_raw(n: i128) -> Option<i64> {
    let abs = if n < 0 { -n } else { n };
    let digits = abs.to_string().len();
    let secs = if digits >= 16 {
        n / 1_000_000
    } else if digits >= 13 {
        n / 1_000
    } else {
        n
    };
    i64::try_from(secs).ok()
}

pub fn epoch_seconds_val(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Number(num)) => num.as_i64().and_then(|x| epoch_seconds_raw(x as i128)),
        Some(Value::String(s)) if s.chars().all(|c| c == '-' || c.is_ascii_digit()) => {
            let val: i128 = s.parse().ok()?;
            epoch_seconds_raw(val)
        }
        _ => None,
    }
}

pub fn group_epoch_bts(group: &[Value]) -> Option<i64> {
    for s in group {
        if let Some(x) = epoch_seconds_val(s.get("Bts")) {
            return Some(x);
        }
        if let Some(b) = s.get("body").and_then(|b| epoch_seconds_val(b.get("Bts"))) {
            return Some(b);
        }
    }
    None
}

pub fn group_epoch_ets(group: &[Value]) -> Option<i64> {
    for s in group {
        if let Some(x) = epoch_seconds_val(s.get("Ets")) {
            return Some(x);
        }
        if let Some(b) = s.get("body").and_then(|b| epoch_seconds_val(b.get("Ets"))) {
            return Some(b);
        }
    }
    None
}

pub fn first_epoch_bts(sections: &[Value]) -> Option<i64> {
    for s in sections {
        if let Some(x) = epoch_seconds_val(s.get("Bts"))
            && x >= 1_000_000_000
        {
            return Some(x);
        }
        if let Some(x) = s.get("body").and_then(|b| epoch_seconds_val(b.get("Bts")))
            && x >= 1_000_000_000
        {
            return Some(x);
        }
    }
    None
}

pub fn first_small_tickstart(sections: &[Value]) -> Option<i64> {
    for s in sections {
        if let Some(ts) = s.get("TickStart").and_then(|v| v.as_i64()) {
            return Some(ts);
        }
        if let Some(ts) = s
            .get("Bts")
            .and_then(|v| v.as_i64())
            .filter(|b| *b < 1_000_000_000)
        {
            return Some(ts);
        }
        if let Some(ts) = s
            .get("body")
            .and_then(|b| b.get("Bts"))
            .and_then(|v| v.as_i64())
            .filter(|b| *b < 1_000_000_000)
        {
            return Some(ts);
        }
    }
    None
}

pub fn small_tick_pair(group: &[Value]) -> Option<(i64, i64)> {
    for s in group {
        let ts = s.get("TickStart").and_then(|v| v.as_i64()).or_else(|| {
            s.get("Bts")
                .and_then(|v| v.as_i64())
                .filter(|b| *b < 1_000_000_000)
        });
        let ets = s
            .get("Ets")
            .and_then(|v| v.as_i64())
            .filter(|e| *e < 1_000_000_000);
        if let (Some(ts), Some(ets)) = (ts, ets)
            && ets >= ts
        {
            return Some((ts, ets));
        }
    }
    for s in group {
        if let (Some(ts), Some(t)) = (
            s.get("TickStart").and_then(|v| v.as_i64()),
            s.get("T")
                .and_then(|v| v.as_i64())
                .filter(|t| *t < 1_000_000_000),
        ) && t > ts
        {
            return Some((ts, ts + (t - ts - 1)));
        }
    }
    None
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

pub fn compose_enemy_hss(
    group: &[Value],
    attack_cid: i64,
    c_idt: &Value,
    enemy_snap: &Value,
) -> String {
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
