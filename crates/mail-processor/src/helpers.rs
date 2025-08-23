use serde_json::{Value, json};

pub fn get_or_insert_object<'a>(obj: &'a mut Value, key: &str) -> &'a mut Value {
    let map = obj.as_object_mut().expect("mail root must be an object");
    if !map.get(key).map(|v| v.is_object()).unwrap_or(false) {
        map.insert(key.to_string(), json!({}));
    }
    map.get_mut(key).unwrap()
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
