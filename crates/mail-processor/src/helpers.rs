use serde_json::{Map, Value};

pub fn get_or_insert_object<'a>(obj: &'a mut Value, key: &str) -> &'a mut Value {
    let map = obj.as_object_mut().expect("mail root must be an object");
    if !map.get(key).map(Value::is_object).unwrap_or(false) {
        map.insert(key.to_string(), Value::Object(Map::new()));
    }
    map.get_mut(key).unwrap()
}

pub fn parse_f64(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn split_semicolons(s: &str) -> impl Iterator<Item = &str> {
    s.split(';').filter(|p| !p.is_empty())
}

pub fn collect_buffs_from_hwbs(hwbs: Option<&Value>) -> String {
    let mut items: Vec<&str> = Vec::new();
    if let Some(obj) = hwbs.and_then(Value::as_object) {
        for v in obj.values() {
            if let Some(b) = v.get("Buffs").and_then(Value::as_str) {
                items.extend(split_semicolons(b));
            }
        }
    }
    items.sort_unstable();
    items.join(";")
}

pub fn collect_affix_from_hwbs(hwbs: Option<&Value>) -> String {
    let mut items: Vec<&str> = Vec::new();
    if let Some(obj) = hwbs.and_then(Value::as_object) {
        for v in obj.values() {
            if let Some(a) = v.get("Affix").and_then(Value::as_str) {
                items.extend(split_semicolons(a));
            }
        }
    }
    items.sort_unstable_by(|a, b| match (a.parse::<i32>(), b.parse::<i32>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    });
    items.join(";")
}

pub fn find_self_snapshot_section(sections: &[Value]) -> Option<&Value> {
    let mut first_appuid: Option<&Value> = None;
    for s in sections {
        if s.get("AppUid").is_some() {
            if s.get("CtId").and_then(Value::as_i64) == Some(0) {
                return Some(s);
            }
            if first_appuid.is_none() {
                first_appuid = Some(s);
            }
        }
    }
    first_appuid
}

pub fn find_self_content_root(sections: &[Value]) -> Option<&Value> {
    for s in sections {
        if s.pointer("/body/content/SelfChar").is_some() || s.pointer("/content/SelfChar").is_some()
        {
            if let Some(v) = s.pointer("/body/content") {
                return Some(v);
            }
            if let Some(v) = s.get("content") {
                return Some(v);
            }
        }
    }
    None
}

pub fn find_attack_block_best_match<'a>(
    group: &'a [Value],
    attack_id: &str,
) -> (Option<usize>, Option<&'a Value>) {
    let mut path = String::with_capacity(9 + attack_id.len());
    path.push_str("/Attacks/");
    path.push_str(attack_id);

    let mut atk_idx: Option<usize> = None;
    let mut atk_val: Option<&Value> = None;
    let mut atk_has_hss = false;
    let mut idt_idx: Option<usize> = None;

    for (i, s) in group.iter().enumerate() {
        if let Some(b) = s.get(attack_id) {
            let is_battleish = b
                .get("Kill")
                .or_else(|| b.get("Damage"))
                .or_else(|| b.get("CIdt"))
                .is_some();
            if is_battleish {
                let has_hss = b.get("CIdt").and_then(|c| c.get("HSS")).is_some();
                if atk_val.is_none() || (!atk_has_hss && has_hss) {
                    atk_idx = Some(i);
                    atk_val = Some(b);
                    atk_has_hss = has_hss;
                }
            }
        } else if let Some(b) = s.pointer(&path) {
            let has_hss = b.get("CIdt").and_then(|c| c.get("HSS")).is_some();
            if atk_val.is_none() || (!atk_has_hss && has_hss) {
                atk_idx = Some(i);
                atk_val = Some(b);
                atk_has_hss = has_hss;
            }
        }

        if idt_idx.is_none() {
            let idt_match = s
                .get("Idt")
                .map(|v| {
                    v.as_str().map(|x| x == attack_id).unwrap_or_else(|| {
                        v.as_i64()
                            .map(|n| {
                                let mut buf = itoa::Buffer::new();
                                buf.format(n) == attack_id
                            })
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);

            if idt_match
                && (s.get("HSS").is_some() || s.get("HId").is_some() || s.get("HId2").is_some())
            {
                idt_idx = Some(i);
            }
        }
    }

    let idx = idt_idx.or(atk_idx);
    (idx, atk_val)
}

pub fn map_put_i64(m: &mut Map<String, Value>, k: &str, v: Option<i64>) {
    if let Some(x) = v {
        m.insert(k.into(), Value::from(x));
    }
}

pub fn map_put_i32(m: &mut Map<String, Value>, k: &str, v: Option<i32>) {
    if let Some(x) = v {
        m.insert(k.into(), Value::from(x));
    }
}

pub fn map_put_f64(m: &mut Map<String, Value>, k: &str, v: Option<f64>) {
    if let Some(x) = v {
        m.insert(k.into(), Value::from(x));
    }
}

pub fn map_put_str(m: &mut Map<String, Value>, k: &str, v: Option<&str>) {
    if let Some(s) = v {
        m.insert(k.into(), Value::String(s.to_owned()));
    }
}

pub fn map_insert_i64_if_absent(m: &mut Map<String, Value>, k: &str, v: Option<i64>) {
    if m.get(k).is_none() {
        map_put_i64(m, k, v);
    }
}

pub fn map_insert_f64_if_absent(m: &mut Map<String, Value>, k: &str, v: Option<f64>) {
    if m.get(k).is_none() {
        map_put_f64(m, k, v);
    }
}

pub fn map_insert_str_if_absent(m: &mut Map<String, Value>, k: &str, v: Option<&str>) {
    if m.get(k).is_none() {
        map_put_str(m, k, v);
    }
}

pub fn extract_avatar_url(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => {
            if let Ok(vv) = serde_json::from_str::<Value>(s)
                && let Some(url) = vv.get("avatar").and_then(Value::as_str)
            {
                return Some(url.to_owned());
            }

            let trimmed = s.trim();
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                return Some(trimmed.to_owned());
            }

            None
        }
        Some(Value::Object(obj)) => obj
            .get("avatar")
            .and_then(Value::as_str)
            .map(|s| s.to_owned()),
        _ => None,
    }
}
