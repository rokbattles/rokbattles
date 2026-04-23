use serde_json::Value;

pub(super) const MAIL_TYPE_SYSTEM_BARBARIAN_FORT: &str = "SystemBarbarianFort";
pub(super) const MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS: &str = "AllianceAOOBattleResults";
pub(super) const MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO: &str = "AllianceAOOBattleInfo";
pub(super) const MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS: &str = "AllianceAOOIndividualResults";

pub(super) fn classify_processable_mail_type(input: &Value) -> Option<&'static str> {
    if is_system_barbarian_fort_mail(input) {
        return Some(MAIL_TYPE_SYSTEM_BARBARIAN_FORT);
    }
    if let Some(mail_type) = detect_alliance_aoo_mail_type(input) {
        return Some(mail_type);
    }

    match input.get("type").and_then(|value| value.as_str()) {
        Some("Battle") => Some("Battle"),
        Some("DuelBattle2") => Some("DuelBattle2"),
        Some("BarCanyonKillBoss") => Some("BarCanyonKillBoss"),
        Some("Rss") => Some("Rss"),
        _ => None,
    }
}

fn detect_alliance_aoo_mail_type(root: &Value) -> Option<&'static str> {
    let root = root.as_object()?;
    if !matches!(root.get("type").and_then(Value::as_str), Some("Alliance")) {
        return None;
    }
    if !matches!(root.get("box").and_then(Value::as_str), Some("AllianceBox")) {
        return None;
    }

    let body = root.get("body").and_then(Value::as_object)?;
    let body_type = body.get("type").and_then(value_as_u64)?;
    let body_param = body.get("param").and_then(value_as_u64);

    match body_type {
        14 if matches!(body_param, Some(1)) => Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS),
        15 if matches!(body_param, Some(1)) => Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS),
        60 => Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS),
        61 => Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO),
        62 => Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS),
        _ => None,
    }
}

fn is_system_barbarian_fort_mail(root: &Value) -> bool {
    let Some(root) = root.as_object() else {
        return false;
    };
    if !matches!(root.get("type").and_then(Value::as_str), Some("System")) {
        return false;
    }
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

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn classify_processable_mail_type_detects_alliance_aoo_variants() {
        let custom_battle_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 14, "param": 1 }
        });
        assert_eq!(
            classify_processable_mail_type(&custom_battle_results),
            Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS)
        );

        let battle_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 60 }
        });
        assert_eq!(
            classify_processable_mail_type(&battle_results),
            Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS)
        );

        let battle_info = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 61 }
        });
        assert_eq!(
            classify_processable_mail_type(&battle_info),
            Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO)
        );

        let custom_individual_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 15, "param": 1 }
        });
        assert_eq!(
            classify_processable_mail_type(&custom_individual_results),
            Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS)
        );

        let individual_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 62 }
        });
        assert_eq!(
            classify_processable_mail_type(&individual_results),
            Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS)
        );
    }

    #[test]
    fn classify_processable_mail_type_rejects_type_14_alliance_mail_with_other_param() {
        let payload = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 14, "param": 2 }
        });
        assert_eq!(classify_processable_mail_type(&payload), None);
    }

    #[test]
    fn classify_processable_mail_type_rejects_type_15_alliance_mail_with_other_param() {
        let payload = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 15, "param": 3 }
        });
        assert_eq!(classify_processable_mail_type(&payload), None);
    }

    #[test]
    fn classify_processable_mail_type_rejects_partial_alliance_matches() {
        let missing_box = json!({
            "type": "Alliance",
            "body": { "type": 60, "param": 1 }
        });
        let missing_body = json!({
            "type": "Alliance",
            "box": "AllianceBox"
        });

        assert_eq!(classify_processable_mail_type(&missing_box), None);
        assert_eq!(classify_processable_mail_type(&missing_body), None);
    }

    #[test]
    fn classify_processable_mail_type_detects_rss() {
        let input = json!({ "type": "Rss" });
        assert_eq!(classify_processable_mail_type(&input), Some("Rss"));
    }

    #[test]
    fn classify_processable_mail_type_detects_system_barbarian_fort_sub_param_three() {
        let input = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 3,
                "subType": 11
            }
        });
        assert_eq!(classify_processable_mail_type(&input), Some(MAIL_TYPE_SYSTEM_BARBARIAN_FORT));
    }

    #[test]
    fn classify_processable_mail_type_rejects_system_mail_with_unsupported_sub_param() {
        let input = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 2,
                "subType": 11
            }
        });
        assert_eq!(classify_processable_mail_type(&input), None);
    }

    #[test]
    fn classify_processable_mail_type_rejects_partial_system_matches() {
        let unsupported_box = json!({
            "type": "System",
            "box": "AllianceBox",
            "body": { "subType": 11, "subParam": 1 }
        });
        let unsupported_sub_type = json!({
            "type": "System",
            "box": "Report",
            "body": { "subType": 10, "subParam": 1 }
        });

        assert_eq!(classify_processable_mail_type(&unsupported_box), None);
        assert_eq!(classify_processable_mail_type(&unsupported_sub_type), None);
    }
}
