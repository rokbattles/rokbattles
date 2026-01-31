use serde_json::{Map, Value};
use std::path::Path;

/// Parse the numeric mail id from a RoK mail filename.
pub(crate) fn parse_rok_mail_id(filename: &str) -> Option<u128> {
    let rest = filename.strip_prefix("Persistent.Mail.")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u128>().ok()
}

/// Normalize a decoded mail payload to a single root object.
///
/// Some mail buffers decode to a singleton array, so we treat that one object
/// as the root for convenience.
fn normalize_mail_root(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(map) => Some(map),
        Value::Array(items) => match items.as_slice() {
            [Value::Object(map)] => Some(map),
            _ => None,
        },
        _ => None,
    }
}

/// Extract the mail type string from a decoded mail payload.
pub(crate) fn detect_mail_type(value: &Value) -> Option<&str> {
    let root = normalize_mail_root(value)?;
    root.get("type").and_then(Value::as_str)
}

/// Check whether a mail type is supported by the upload pipeline.
pub(crate) fn is_supported_mail_type(mail_type: &str) -> bool {
    mail_type.eq_ignore_ascii_case("Battle")
        || mail_type.eq_ignore_ascii_case("DuelBattle2")
        || mail_type.eq_ignore_ascii_case("BarCanyonKillBoss")
}

/// Heuristic header validation to quickly skip non-mail buffers.
pub(crate) fn has_rok_mail_header(buf: &[u8]) -> bool {
    if buf.len() < 32 {
        return false;
    }
    if buf[0] != 0xFF {
        return false;
    }
    if buf[9] != 0x05 || buf[10] != 0x04 {
        return false;
    }
    let len = {
        let start = 11;
        let end = start + 4;
        let Some(bytes) = buf.get(start..end) else {
            return false;
        };
        u32::from_le_bytes(bytes.try_into().unwrap_or([0; 4]))
    };
    if len != 9 {
        return false;
    }
    let start = 15;
    let end = start + 9;
    let Some(bytes) = buf.get(start..end) else {
        return false;
    };
    bytes == b"mailScene"
}

/// Extract a non-empty file name for API uploads.
pub(crate) fn file_name_for_upload(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|s| s.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_mail_root_accepts_object_and_singleton_array() {
        let object = json!({ "type": "Battle" });
        assert!(normalize_mail_root(&object).is_some());

        let singleton = json!([{ "type": "Battle" }]);
        assert!(normalize_mail_root(&singleton).is_some());

        let multiple = json!([{ "type": "Battle" }, { "type": "Battle" }]);
        assert!(normalize_mail_root(&multiple).is_none());
    }

    #[test]
    fn detect_mail_type_pulls_string_type() {
        let payload = json!({ "type": "DuelBattle2" });
        assert_eq!(detect_mail_type(&payload), Some("DuelBattle2"));

        let non_string = json!({ "type": 12 });
        assert_eq!(detect_mail_type(&non_string), None);
    }

    #[test]
    fn supported_mail_types_are_case_insensitive() {
        assert!(is_supported_mail_type("Battle"));
        assert!(is_supported_mail_type("duelbattle2"));
        assert!(is_supported_mail_type("BARCANYONKILLBOSS"));
        assert!(!is_supported_mail_type("Unknown"));
    }

    #[test]
    fn parse_rok_mail_id_requires_numeric_suffix() {
        assert_eq!(parse_rok_mail_id("Persistent.Mail.123"), Some(123));
        assert_eq!(parse_rok_mail_id("Persistent.Mail.001"), Some(1));
        assert_eq!(parse_rok_mail_id("Persistent.Mail."), None);
        assert_eq!(parse_rok_mail_id("Persistent.Mail.123a"), None);
        assert_eq!(parse_rok_mail_id("Other.Mail.123"), None);
    }

    #[test]
    fn file_name_for_upload_rejects_missing_names() {
        assert_eq!(
            file_name_for_upload(Path::new("Persistent.Mail.123")),
            Some("Persistent.Mail.123".to_string())
        );
        assert_eq!(file_name_for_upload(Path::new("")), None);
    }

    #[test]
    fn has_rok_mail_header_matches_expected_bytes() {
        let mut buf = vec![0u8; 32];
        buf[0] = 0xFF;
        buf[9] = 0x05;
        buf[10] = 0x04;
        buf[11..15].copy_from_slice(9u32.to_le_bytes().as_slice());
        buf[15..24].copy_from_slice(b"mailScene");

        assert!(has_rok_mail_header(&buf));

        let mut wrong = buf.clone();
        wrong[0] = 0x00;
        assert!(!has_rok_mail_header(&wrong));
    }
}
