use serde_json::Value;

pub(crate) fn parse_i128(v: &Value) -> Option<i128> {
    match v {
        Value::Number(n) => n
            .as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from)),
        Value::String(s) => s.trim().parse::<i128>().ok(),
        _ => None,
    }
}

pub(crate) fn parse_i64(v: &Value) -> Option<i64> {
    parse_i128(v).and_then(|n| i64::try_from(n).ok())
}

pub(crate) fn parse_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|x| x != 0),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn parse_avatar(value: Option<&Value>) -> (Option<String>, Option<String>) {
    let Some(value) = value else {
        return (None, None);
    };

    match value {
        Value::String(raw) => {
            if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
                return (
                    parsed
                        .get("avatar")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    parsed
                        .get("avatarFrame")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                );
            }

            let trimmed = raw.trim();
            if trimmed.is_empty() {
                (None, None)
            } else {
                (Some(trimmed.to_owned()), None)
            }
        }
        Value::Object(map) => (
            map.get("avatar").and_then(Value::as_str).map(str::to_owned),
            map.get("avatarFrame")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_avatar;
    use serde_json::json;

    #[test]
    fn parse_avatar_reads_json_string_payload() {
        let value = json!("{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}");

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert_eq!(avatar_url.as_deref(), Some("http://a"));
        assert_eq!(frame_url.as_deref(), Some("http://f"));
    }

    #[test]
    fn parse_avatar_accepts_plain_url_string() {
        let value = json!("https://example.com/avatar.png");

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert_eq!(
            avatar_url.as_deref(),
            Some("https://example.com/avatar.png")
        );
        assert!(frame_url.is_none());
    }

    #[test]
    fn parse_avatar_ignores_empty_string() {
        let value = json!("   ");

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert!(avatar_url.is_none());
        assert!(frame_url.is_none());
    }

    #[test]
    fn parse_avatar_reads_object_payload() {
        let value = json!({
            "avatar": "http://b",
            "avatarFrame": "http://g"
        });

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert_eq!(avatar_url.as_deref(), Some("http://b"));
        assert_eq!(frame_url.as_deref(), Some("http://g"));
    }

    #[test]
    fn parse_avatar_skips_non_string_objects() {
        let value = json!(123);

        let (avatar_url, frame_url) = parse_avatar(Some(&value));

        assert!(avatar_url.is_none());
        assert!(frame_url.is_none());
    }
}
