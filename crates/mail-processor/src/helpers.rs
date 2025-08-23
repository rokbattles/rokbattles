use serde_json::{Map, Value};

pub fn get_or_insert_object<'a>(obj: &'a mut Value, key: &str) -> &'a mut Value {
    let map = obj.as_object_mut().expect("mail root must be an object");
    if !map.get(key).map(Value::is_object).unwrap_or(false) {
        map.insert(key.to_string(), Value::Object(Map::new()));
    }
    map.get_mut(key).unwrap()
}

pub fn pick_f64(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn split_semicolons(s: &str) -> impl Iterator<Item = &str> {
    s.split(';').filter(|p| !p.is_empty())
}

pub fn join_buffs(hwbs: Option<&Value>) -> String {
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

pub fn join_affix(hwbs: Option<&Value>) -> String {
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

pub fn find_self_snapshot_ref(sections: &[Value]) -> Option<&Value> {
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

pub fn find_self_snapshot(sections: &[Value]) -> Value {
    find_self_snapshot_ref(sections)
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn find_self_body_ref(sections: &[Value]) -> Option<&Value> {
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

pub fn find_self_body(sections: &[Value]) -> Value {
    find_self_body_ref(sections).cloned().unwrap_or(Value::Null)
}

pub fn find_best_attack_block_ref<'a>(
    group: &'a [Value],
    attack_id: &str,
) -> (Option<usize>, Option<&'a Value>) {
    let mut path = String::with_capacity(9 + attack_id.len());
    path.push_str("/Attacks/");
    path.push_str(attack_id);

    let mut best_idx: Option<usize> = None;
    let mut best_val: Option<&Value> = None;
    let mut best_has_hss = false;

    for (i, s) in group.iter().enumerate() {
        if let Some(b) = s.get(attack_id).or_else(|| s.pointer(&path)) {
            let has_hss = b.get("CIdt").and_then(|c| c.get("HSS")).is_some();
            if best_val.is_none() || (!best_has_hss && has_hss) {
                best_idx = Some(i);
                best_val = Some(b);
                best_has_hss = has_hss;
                if best_has_hss {
                    break;
                }
            }
        }
    }

    if best_val.is_none() {
        for (i, s) in group.iter().enumerate() {
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
                return (Some(i), None);
            }
        }
    }

    (best_idx, best_val)
}

pub fn find_best_attack_block(group: &[Value], attack_id: &str) -> (Option<usize>, Value) {
    match find_best_attack_block_ref(group, attack_id) {
        (idx, Some(v)) => (idx, v.clone()),
        (idx, None) => (idx, Value::Object(Map::new())),
    }
}
