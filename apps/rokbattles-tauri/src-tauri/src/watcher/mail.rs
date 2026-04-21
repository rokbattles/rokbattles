use std::path::Path;

use serde_json::{Map, Value};

const SYSTEM_BARBARIAN_FORT_MAIL_TYPE: &str = "SystemBarbarianFort";
const ALLIANCE_AOO_BATTLE_RESULTS_MAIL_TYPE: &str = "AllianceAOOBattleResults";
const ALLIANCE_AOO_BATTLE_INFO_MAIL_TYPE: &str = "AllianceAOOBattleInfo";
const ALLIANCE_AOO_INDIVIDUAL_RESULTS_MAIL_TYPE: &str = "AllianceAOOIndividualResults";

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

/// Detect a supported mail type, including typed System and Alliance variants.
pub(crate) fn detect_supported_mail_type(value: &Value) -> Option<&'static str> {
    let root = normalize_mail_root(value)?;
    let mail_type = root.get("type").and_then(Value::as_str)?;

    if let Some(canonical) = canonical_supported_mail_type(mail_type) {
        return Some(canonical);
    }

    if mail_type.eq_ignore_ascii_case("System") && is_system_barbarian_fort_mail(root) {
        return Some(SYSTEM_BARBARIAN_FORT_MAIL_TYPE);
    }
    if mail_type.eq_ignore_ascii_case("Alliance")
        && let Some(alliance_aoo_type) = detect_alliance_aoo_mail_type(root)
    {
        return Some(alliance_aoo_type);
    }

    None
}

fn canonical_supported_mail_type(mail_type: &str) -> Option<&'static str> {
    if mail_type.eq_ignore_ascii_case("Battle") {
        return Some("Battle");
    }
    if mail_type.eq_ignore_ascii_case("DuelBattle2") {
        return Some("DuelBattle2");
    }
    if mail_type.eq_ignore_ascii_case("BarCanyonKillBoss") {
        return Some("BarCanyonKillBoss");
    }
    if mail_type.eq_ignore_ascii_case("Rss") {
        return Some("Rss");
    }
    if mail_type.eq_ignore_ascii_case(SYSTEM_BARBARIAN_FORT_MAIL_TYPE) {
        return Some(SYSTEM_BARBARIAN_FORT_MAIL_TYPE);
    }
    None
}

fn is_system_barbarian_fort_mail(root: &Map<String, Value>) -> bool {
    if !matches!(root.get("box").and_then(Value::as_str), Some("Report")) {
        return false;
    }

    let Some(body) = root.get("body").and_then(Value::as_object) else {
        return false;
    };

    let sub_param = body.get("subParam").and_then(value_as_u64);
    let sub_type = body.get("subType").and_then(value_as_u64);
    matches!(sub_type, Some(11)) && matches!(sub_param, Some(1 | 3))
}

fn detect_alliance_aoo_mail_type(root: &Map<String, Value>) -> Option<&'static str> {
    if !matches!(root.get("box").and_then(Value::as_str), Some("AllianceBox")) {
        return None;
    }

    let body = root.get("body").and_then(Value::as_object)?;
    let body_type = body.get("type").and_then(value_as_u64)?;
    let body_param = body.get("param").and_then(value_as_u64);

    match body_type {
        // custom Ark match
        14 if matches!(body_param, Some(1)) => Some(ALLIANCE_AOO_BATTLE_RESULTS_MAIL_TYPE),
        15 => Some(ALLIANCE_AOO_INDIVIDUAL_RESULTS_MAIL_TYPE),
        // normal Ark match
        60 => Some(ALLIANCE_AOO_BATTLE_RESULTS_MAIL_TYPE),
        61 => Some(ALLIANCE_AOO_BATTLE_INFO_MAIL_TYPE),
        62 => Some(ALLIANCE_AOO_INDIVIDUAL_RESULTS_MAIL_TYPE),
        _ => None,
    }
}

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
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
    path.file_name().and_then(|s| s.to_str()).filter(|name| !name.is_empty()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
    fn detect_supported_mail_type_matches_system_barbarian_fort() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 1,
                "subType": 11
            }
        });
        assert_eq!(detect_supported_mail_type(&payload), Some("SystemBarbarianFort"));
    }

    #[test]
    fn detect_supported_mail_type_matches_system_barbarian_fort_sub_param_three() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 3,
                "subType": 11
            }
        });
        assert_eq!(detect_supported_mail_type(&payload), Some("SystemBarbarianFort"));
    }

    #[test]
    fn detect_supported_mail_type_rejects_other_system_mail() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 1,
                "subType": 10
            }
        });
        assert_eq!(detect_supported_mail_type(&payload), None);
    }

    #[test]
    fn detect_supported_mail_type_rejects_unsupported_system_sub_param() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 2,
                "subType": 11
            }
        });
        assert_eq!(detect_supported_mail_type(&payload), None);
    }

    #[test]
    fn detect_supported_mail_type_matches_alliance_aoo_variants() {
        let custom_battle_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 14, "param": 1 }
        });
        assert_eq!(
            detect_supported_mail_type(&custom_battle_results),
            Some("AllianceAOOBattleResults")
        );

        let battle_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 60 }
        });
        assert_eq!(detect_supported_mail_type(&battle_results), Some("AllianceAOOBattleResults"));

        let battle_info = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 61 }
        });
        assert_eq!(detect_supported_mail_type(&battle_info), Some("AllianceAOOBattleInfo"));

        let custom_individual_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 15 }
        });
        assert_eq!(
            detect_supported_mail_type(&custom_individual_results),
            Some("AllianceAOOIndividualResults")
        );

        let individual_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 62 }
        });
        assert_eq!(
            detect_supported_mail_type(&individual_results),
            Some("AllianceAOOIndividualResults")
        );
    }

    #[test]
    fn detect_supported_mail_type_rejects_type_14_alliance_mail_with_other_param() {
        let payload = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 14, "param": 2 }
        });
        assert_eq!(detect_supported_mail_type(&payload), None);
    }

    #[test]
    fn detect_supported_mail_type_rejects_non_aoo_alliance_mail() {
        let payload = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 99 }
        });
        assert_eq!(detect_supported_mail_type(&payload), None);
    }

    #[test]
    fn supported_mail_types_are_case_insensitive() {
        assert_eq!(detect_supported_mail_type(&json!({ "type": "Battle" })), Some("Battle"));
        assert_eq!(
            detect_supported_mail_type(&json!({ "type": "duelbattle2" })),
            Some("DuelBattle2")
        );
        assert_eq!(
            detect_supported_mail_type(&json!({ "type": "BARCANYONKILLBOSS" })),
            Some("BarCanyonKillBoss")
        );
        assert_eq!(detect_supported_mail_type(&json!({ "type": "rss" })), Some("Rss"));
        assert_eq!(
            detect_supported_mail_type(&json!({ "type": "systembarbarianfort" })),
            Some("SystemBarbarianFort")
        );
        assert_eq!(
            detect_supported_mail_type(&json!({
                "type": "alliance",
                "box": "AllianceBox",
                "body": { "type": 60 }
            })),
            Some("AllianceAOOBattleResults")
        );
        assert_eq!(detect_supported_mail_type(&json!({ "type": "Unknown" })), None);
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
