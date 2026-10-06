//! Mail identity and classification shared by both upload paths and document construction.

use serde_json::Value;

use crate::error::ApiError;

/// Fields required to store a decoded mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MailMetadata {
    pub id: String,
    pub time: i64,
    pub receiver: String,
}

/// Resolves a registered mail category or returns the unrecognized type label.
///
/// Returns an error if the type is missing or a recognized label fails the
/// registry's subtype checks. An unrecognized label is not necessarily supported;
/// callers must check it before accepting the mail.
pub(crate) fn extract_mail_type(decoded: &Value) -> Result<String, ApiError> {
    if let Some(mail_type) = rokbattles_mail_registry::detect_mail_type(decoded) {
        return Ok(mail_type.to_string());
    }

    let raw = rokbattles_mail_registry::raw_mail_type_string(decoded)
        .ok_or_else(|| ApiError::bad_request("missing mail type"))?;
    if rokbattles_mail_registry::MailType::from_label_ignore_ascii_case(&raw).is_some() {
        return Err(ApiError::unsupported_type(raw));
    }

    Ok(raw)
}

/// Reads the first string or numeric ID from `id`, `mail_id`, or `metadata.mail_id`.
///
/// Checks fields in that order, skipping missing values and other JSON types.
/// Numbers become strings. Returns `None` if the root is not an object or no ID matches.
pub(crate) fn extract_mail_id(decoded: &Value) -> Option<String> {
    let root = rokbattles_mail_registry::normalize_mail_root(decoded)?;

    root.get("id")
        .and_then(value_to_string)
        .or_else(|| root.get("mail_id").and_then(value_to_string))
        .or_else(|| {
            root.get("metadata").and_then(|meta| meta.get("mail_id")).and_then(value_to_string)
        })
}

/// Extracts the mail ID, timestamp, and receiver required for storage.
///
/// Uses [`extract_mail_id`] for the ID and falls back from `time` to
/// `metadata.mail_time` for the timestamp. Returns an error for non-object roots,
/// missing IDs, timestamps that are not integers representable as `i64`, or
/// receivers without the `player_` prefix.
pub(crate) fn extract_metadata(decoded: &Value) -> Result<MailMetadata, ApiError> {
    let root = rokbattles_mail_registry::normalize_mail_root(decoded)
        .ok_or_else(|| ApiError::bad_request("invalid mail root"))?;
    let object = root.as_object().ok_or_else(|| ApiError::bad_request("invalid mail root"))?;

    let id = extract_mail_id(root).ok_or_else(|| ApiError::bad_request("missing mail id"))?;
    let time = object
        .get("time")
        .and_then(value_to_i64)
        .or_else(|| {
            object.get("metadata").and_then(|meta| meta.get("mail_time")).and_then(value_to_i64)
        })
        .ok_or_else(|| ApiError::bad_request("missing mail time"))?;
    let receiver = extract_receiver_identity(object)?;

    Ok(MailMetadata { id, time, receiver })
}

fn extract_receiver_identity(object: &serde_json::Map<String, Value>) -> Result<String, ApiError> {
    object
        .get("receiver")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with("player_"))
        .map(str::to_string)
        .ok_or_else(|| ApiError::bad_request("missing mail receiver"))
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(value) => {
            value.as_i64().or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use rokbattles_mail_registry::is_supported_mail_type;
    use serde_json::json;

    use super::*;

    #[test]
    fn metadata_uses_the_same_id_precedence_as_upload_validation() {
        let decoded = json!({
            "id": 12345,
            "mail_id": "other",
            "metadata": { "mail_id": "fallback" },
            "time": 42,
            "receiver": "player_22"
        });

        assert_eq!(extract_mail_id(&decoded).as_deref(), Some("12345"));
        assert_eq!(extract_metadata(&decoded).unwrap().id, "12345");
    }

    #[test]
    fn metadata_falls_back_when_top_level_fields_are_unusable() {
        let decoded = json!({
            "id": null,
            "mail_id": false,
            "time": "not a timestamp",
            "metadata": { "mail_id": 12345, "mail_time": 42 },
            "receiver": "player_22"
        });

        assert_eq!(extract_mail_id(&decoded).as_deref(), Some("12345"));
        assert_eq!(
            extract_metadata(&decoded).unwrap(),
            MailMetadata { id: "12345".to_string(), time: 42, receiver: "player_22".to_string() }
        );
    }

    #[test]
    fn extracts_mail_type() {
        let decoded = json!({ "type": "Battle" });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Battle");
    }

    #[test]
    fn rejects_mail_type_from_singleton_array() {
        let decoded = json!([{ "type": "Battle" }]);
        extract_mail_type(&decoded).expect_err("input should be rejected");
    }

    #[test]
    fn extracts_gve_member_loot_report_and_rejects_other_events() {
        let gve = json!({
            "type": "EventMemberLootReport",
            "body": { "content": { "EventName": "GVE" } }
        });
        assert_eq!(extract_mail_type(&gve).unwrap(), "EventMemberLootReport");

        let other = json!({
            "type": "EventMemberLootReport",
            "body": { "content": { "EventName": "OtherEvent" } }
        });
        assert!(matches!(extract_mail_type(&other), Err(ApiError::UnsupportedType(_))));
    }

    #[test]
    fn supports_known_mail_types() {
        assert!(is_supported_mail_type("Battle"));
        assert!(is_supported_mail_type("DuelBattle2"));
        assert!(is_supported_mail_type("BarCanyonKillBoss"));
        assert!(is_supported_mail_type("KillEliteBarReport"));
        assert!(is_supported_mail_type("EventMemberLootReport"));
        assert!(is_supported_mail_type("Rss"));
        assert!(is_supported_mail_type("SystemBarbarianFort"));
        assert!(is_supported_mail_type("SystemKaharTreasure"));
        assert!(is_supported_mail_type("AllianceAOOBattleResults"));
        assert!(is_supported_mail_type("AllianceAOOBattleInfo"));
        assert!(is_supported_mail_type("AllianceAOOIndividualResults"));
        assert!(!is_supported_mail_type("Unknown"));
    }

    #[test]
    fn extracts_system_barbarian_fort_mail_type() {
        let decoded = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 1,
                "subType": 11
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "SystemBarbarianFort".to_string());
    }

    #[test]
    fn extracts_system_barbarian_fort_mail_type_with_sub_param_three() {
        let decoded = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 3,
                "subType": 11
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "SystemBarbarianFort".to_string());
    }

    #[test]
    fn extracts_system_motte_mail_type() {
        let decoded = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 4,
                "subType": 11
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "SystemBarbarianFort".to_string());
    }

    #[test]
    fn extracts_system_kahar_treasure_mail_type() {
        let decoded = json!({
            "type": "System",
            "box": "SystemBox",
            "body": {
                "subParam": 11,
                "subType": 29
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "SystemKaharTreasure".to_string());
    }

    #[test]
    fn keeps_regular_system_mail_type_unmodified() {
        let decoded = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 1,
                "subType": 10
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "System");
    }

    #[test]
    fn keeps_system_mail_type_unmodified_for_unsupported_sub_param() {
        let decoded = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 2,
                "subType": 11
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "System");
    }

    #[test]
    fn extracts_alliance_aoo_battle_results_mail_type() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 60
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "AllianceAOOBattleResults".to_string());
    }

    #[test]
    fn extracts_alliance_type_14_battle_results_mail_type() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 14,
                "param": 1
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "AllianceAOOBattleResults".to_string());
    }

    #[test]
    fn keeps_type_14_alliance_mail_when_param_is_not_one() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 14,
                "param": 2
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Alliance".to_string());
    }

    #[test]
    fn extracts_alliance_aoo_battle_info_mail_type() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 61
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "AllianceAOOBattleInfo".to_string());
    }

    #[test]
    fn extracts_alliance_aoo_individual_results_mail_type() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 62
            }
        });
        assert_eq!(
            extract_mail_type(&decoded).unwrap(),
            "AllianceAOOIndividualResults".to_string()
        );
    }

    #[test]
    fn extracts_alliance_type_15_individual_results_mail_type() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 15,
                "param": 1
            }
        });
        assert_eq!(
            extract_mail_type(&decoded).unwrap(),
            "AllianceAOOIndividualResults".to_string()
        );
    }

    #[test]
    fn keeps_type_15_alliance_mail_when_param_is_not_one() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 15,
                "param": 3
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Alliance".to_string());
    }

    #[test]
    fn keeps_regular_alliance_mail_type_unmodified() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 99
            }
        });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Alliance");
    }

    #[test]
    fn extracts_mail_id_from_id() {
        let decoded = json!({ "id": "12345" });
        assert_eq!(extract_mail_id(&decoded).as_deref(), Some("12345"));
    }

    #[test]
    fn rejects_mail_id_from_singleton_array() {
        let decoded = json!([{ "id": "12345" }]);
        assert_eq!(extract_mail_id(&decoded), None);
    }

    #[test]
    fn extracts_mail_id_from_mail_id() {
        let decoded = json!({ "mail_id": "999" });
        assert_eq!(extract_mail_id(&decoded).as_deref(), Some("999"));
    }

    #[test]
    fn extracts_mail_id_from_metadata() {
        let decoded = json!({ "metadata": { "mail_id": "meta-1" } });
        assert_eq!(extract_mail_id(&decoded).as_deref(), Some("meta-1"));
    }

    #[test]
    fn extracts_v2_metadata_from_decoded_mail() {
        let decoded = json!({
            "id": "12345",
            "time": 1772127772844751_u64,
            "sender": "system",
            "receiver": "player_71738515"
        });

        let metadata = extract_metadata(&decoded).expect("metadata");

        assert_eq!(
            metadata,
            MailMetadata {
                id: "12345".to_string(),
                time: 1772127772844751,
                receiver: "player_71738515".to_string(),
            }
        );
    }

    #[test]
    fn rejects_v2_metadata_from_singleton_array() {
        let decoded = json!([{
            "id": "12345",
            "time": 1772127772844751_u64,
            "sender": "player_11",
            "receiver": "player_22"
        }]);

        extract_metadata(&decoded).expect_err("input should be rejected");
    }
}
