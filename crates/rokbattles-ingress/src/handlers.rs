use std::{io::Cursor, sync::Arc};

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use bytes::Bytes;
use mongodb::bson::{Binary, Bson, DateTime, doc, spec::BinarySubtype};
use serde::Serialize;
use serde_json::Value;

use crate::{
    clamav::{ScanStatus, scan_zstream},
    error::ApiError,
    state::AppState,
};

const STATUS_PENDING: &str = "pending";
const STATUS_REPROCESS: &str = "reprocess";
const STATUS_UNPROCESSABLE: &str = "unprocessable";
const MAIL_TYPE_SYSTEM_BARBARIAN_FORT: &str = "SystemBarbarianFort";
const MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS: &str = "AllianceAOOBattleResults";
const MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO: &str = "AllianceAOOBattleInfo";
const MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS: &str = "AllianceAOOIndividualResults";

/// Response payload returned from the upload endpoint.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    status: String,
    mail_id: String,
    mail_type: String,
    mail_attack_count: i64,
}

#[derive(Debug, Clone, Copy)]
enum UploadAction {
    Insert,
    Update,
    Skip,
}

#[derive(Debug)]
struct UploadInput {
    bytes: Bytes,
    file_name: String,
    file_id: String,
}

/// Liveness check endpoint.
pub async fn health() -> StatusCode {
    StatusCode::OK
}

/// Accept a mail report upload and persist it if it's new or newer.
pub async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let user_agent = extract_user_agent(&headers)?;
    let upload = read_upload(&mut multipart).await?;
    let buffer = upload.bytes;

    if state.config.clamav_enabled {
        let timeout = std::time::Duration::from_millis(state.config.clamav_timeout_ms);
        match scan_zstream(&buffer, &state.config.clamav_addr, timeout).await {
            Ok(ScanStatus::Clean) => {}
            Ok(ScanStatus::Infected(reason)) => {
                return Err(ApiError::bad_request(format!("clamav detected malware: {reason}")));
            }
            Err(error) => {
                return Err(ApiError::clamav(error.to_string()));
            }
        }
    }

    let decoded = mail_decoder::decode(&buffer)
        .map_err(|error| ApiError::decode_failed(error.to_string()))?;

    let mail_type = extract_mail_type(&decoded)?;
    if !is_supported_mail_type(mail_type.as_str()) {
        return Err(ApiError::unsupported_type(mail_type));
    }

    let mail_id =
        extract_mail_id(&decoded).ok_or_else(|| ApiError::bad_request("missing mail id"))?;
    if mail_id != upload.file_id {
        return Err(ApiError::bad_request(format!(
            "mail id mismatch (payload {mail_id}, filename {})",
            upload.file_name
        )));
    }

    let attack_count = count_attacks(&decoded) as i64;

    let existing = state
        .storage
        .find_existing(&mail_id)
        .await
        .map_err(|error| ApiError::database(error.to_string()))?;

    let action = decide_action(existing.as_ref().map(|entry| entry.attack_count), attack_count);

    if matches!(action, UploadAction::Insert | UploadAction::Update) {
        let compressed = compress_mail_value(&decoded, state.config.zstd_level)?;
        let now = DateTime::now();

        let lossless_doc = decode_lossless_doc(&buffer)?;
        let lossless_compressed = compress_mail_value(&lossless_doc, state.config.zstd_level)?;

        match action {
            UploadAction::Insert => {
                let raw_doc = doc! {
                    "mail_id": &mail_id,
                    "mail_attack_count": attack_count,
                    "user_agent": &user_agent,
                    "status": insert_status_for_mail_type(&mail_type),
                    "mail_value": Bson::Binary(Binary {
                        subtype: BinarySubtype::Generic,
                        bytes: compressed,
                    }),
                    "createdAt": now,
                    "updatedAt": now,
                };
                state
                    .storage
                    .insert_raw(raw_doc)
                    .await
                    .map_err(|error| ApiError::database(error.to_string()))?;

                let lossless_doc = doc! {
                    "mail_id": &mail_id,
                    "mail_attack_count": attack_count,
                    "user_agent": &user_agent,
                    "mail_value": Bson::Binary(Binary {
                        subtype: BinarySubtype::Generic,
                        bytes: lossless_compressed,
                    }),
                    "createdAt": now,
                    "updatedAt": now,
                };
                state
                    .storage
                    .insert_lossless(lossless_doc)
                    .await
                    .map_err(|error| ApiError::database(error.to_string()))?;
            }
            UploadAction::Update => {
                let raw_update = doc! {
                    "mail_attack_count": attack_count,
                    "user_agent": &user_agent,
                    "status": update_status_for_mail_type(&mail_type),
                    "mail_value": Bson::Binary(Binary {
                        subtype: BinarySubtype::Generic,
                        bytes: compressed,
                    }),
                    "updatedAt": now,
                };
                state
                    .storage
                    .update_raw(&mail_id, raw_update)
                    .await
                    .map_err(|error| ApiError::database(error.to_string()))?;

                let lossless_update = doc! {
                    "mail_attack_count": attack_count,
                    "user_agent": &user_agent,
                    "mail_value": Bson::Binary(Binary {
                        subtype: BinarySubtype::Generic,
                        bytes: lossless_compressed,
                    }),
                    "updatedAt": now,
                };
                state
                    .storage
                    .update_lossless(&mail_id, lossless_update)
                    .await
                    .map_err(|error| ApiError::database(error.to_string()))?;
            }
            UploadAction::Skip => {}
        }
    }

    let (status, label) = match action {
        UploadAction::Insert => (StatusCode::CREATED, "stored"),
        UploadAction::Update => (StatusCode::OK, "updated"),
        UploadAction::Skip => (StatusCode::OK, "skipped"),
    };

    let response = UploadResponse {
        status: label.to_string(),
        mail_id,
        mail_type,
        mail_attack_count: attack_count,
    };

    Ok((status, Json(response)))
}

async fn read_upload(multipart: &mut Multipart) -> Result<UploadInput, ApiError> {
    while let Some(field) =
        multipart.next_field().await.map_err(|error| ApiError::bad_request(error.to_string()))?
    {
        if field.file_name().is_some() || field.name().is_some() {
            let file_name = field
                .file_name()
                .map(|name| name.to_string())
                .ok_or_else(|| ApiError::bad_request("missing file name"))?;
            let file_id = parse_mail_id_from_filename(&file_name)?;
            let content_type = field
                .content_type()
                .ok_or_else(|| ApiError::bad_request("missing content type"))?;
            if !is_allowed_content_type(content_type) {
                return Err(ApiError::bad_request(format!(
                    "unsupported content type: {content_type}"
                )));
            }
            if !is_allowed_content_encoding(field.headers().get("content-encoding")) {
                return Err(ApiError::bad_request(
                    "unsupported content encoding (must be identity)",
                ));
            }
            let bytes =
                field.bytes().await.map_err(|error| ApiError::bad_request(error.to_string()))?;
            if !bytes.is_empty() {
                if is_probably_json(&bytes) {
                    return Err(ApiError::bad_request(
                        "expected binary mail buffer, received JSON",
                    ));
                }
                return Ok(UploadInput { bytes, file_name, file_id });
            }
        }
    }

    Err(ApiError::bad_request("missing upload file"))
}

fn extract_mail_type(decoded: &Value) -> Result<String, ApiError> {
    let root = normalize_root(decoded).ok_or_else(|| ApiError::bad_request("missing mail type"))?;
    let mail_type = root
        .get("type")
        .and_then(value_to_string)
        .ok_or_else(|| ApiError::bad_request("missing mail type"))?;

    if is_system_barbarian_fort_mail(root) {
        return Ok(MAIL_TYPE_SYSTEM_BARBARIAN_FORT.to_string());
    }
    if let Some(alliance_aoo_type) = detect_alliance_aoo_mail_type(root) {
        return Ok(alliance_aoo_type.to_string());
    }

    Ok(mail_type)
}

fn is_supported_mail_type(mail_type: &str) -> bool {
    matches!(
        mail_type,
        "Battle"
            | "DuelBattle2"
            | "BarCanyonKillBoss"
            | "Rss"
            | MAIL_TYPE_SYSTEM_BARBARIAN_FORT
            | MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS
            | MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO
            | MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS
    )
}

fn is_processable_mail_type(mail_type: &str) -> bool {
    matches!(
        mail_type,
        "Battle"
            | "DuelBattle2"
            | "BarCanyonKillBoss"
            | "Rss"
            | MAIL_TYPE_SYSTEM_BARBARIAN_FORT
            | MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS
            | MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO
            | MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS
    )
}

fn insert_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_PENDING } else { STATUS_UNPROCESSABLE }
}

fn update_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_REPROCESS } else { STATUS_UNPROCESSABLE }
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
        // custom Ark match
        14 if matches!(body_param, Some(1)) => Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS),
        15 => Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS),
        // normal Ark match
        60 => Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS),
        61 => Some(MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO),
        62 => Some(MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS),
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

fn extract_mail_id(decoded: &Value) -> Option<String> {
    let root = normalize_root(decoded)?;
    root.get("id")
        .and_then(value_to_string)
        .or_else(|| root.get("mail_id").and_then(value_to_string))
        .or_else(|| {
            root.get("metadata").and_then(|meta| meta.get("mail_id")).and_then(value_to_string)
        })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Normalize the decoded mail payload to a single root object.
///
/// Some mail samples encode as a singleton array; in that case we treat the sole
/// object as the root. Any other shape is rejected.
fn normalize_root(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(_) => Some(value),
        Value::Array(items) => match items.as_slice() {
            [item] if item.is_object() => Some(item),
            _ => None,
        },
        _ => None,
    }
}

fn parse_mail_id_from_filename(file_name: &str) -> Result<String, ApiError> {
    let base = std::path::Path::new(file_name)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ApiError::bad_request("invalid file name"))?;

    let prefix = "Persistent.Mail.";
    let rest = base
        .strip_prefix(prefix)
        .ok_or_else(|| ApiError::bad_request("filename must start with Persistent.Mail.<ID>"))?;
    let id: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if id.is_empty() {
        return Err(ApiError::bad_request("filename must include numeric mail id"));
    }
    Ok(id)
}

fn is_allowed_content_type(content_type: &str) -> bool {
    content_type.eq_ignore_ascii_case("application/octet-stream")
}

fn is_allowed_content_encoding(value: Option<&HeaderValue>) -> bool {
    let Some(value) = value else {
        return true;
    };
    value.to_str().map(|value| value.eq_ignore_ascii_case("identity")).unwrap_or(false)
}

/// Extracts and validates the user agent header.
///
/// Expected format: `ROKBattles/<version>` with an optional suffix containing `Tauri/`.
fn extract_user_agent(headers: &HeaderMap) -> Result<String, ApiError> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::bad_request("missing user-agent"))?;

    if !ua_ok(user_agent) {
        return Err(ApiError::bad_request("bad user agent"));
    }

    Ok(user_agent.to_string())
}

/// Validates the `ROKBattles/<version>` prefix and optional Tauri suffix.
fn ua_ok(user_agent: &str) -> bool {
    let Some(rest) = user_agent.strip_prefix("ROKBattles/") else {
        return false;
    };

    let mut parts = rest.splitn(2, ' ');
    let version = parts.next().unwrap_or_default();
    if version.is_empty() {
        return false;
    }

    match parts.next() {
        None => true,
        Some(remainder) => remainder.starts_with('(') && remainder.contains(" Tauri/"),
    }
}

fn is_probably_json(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(256);
    let sample = &bytes[..sample_len];
    let Ok(text) = std::str::from_utf8(sample) else {
        return false;
    };
    let trimmed = text.trim_start_matches(|ch: char| ch.is_whitespace() || ch == '\u{feff}');
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn count_attacks(value: &Value) -> usize {
    find_attacks_object(value).map_or(0, |attacks| attacks.len())
}

fn decide_action(existing_attack_count: Option<i64>, attack_count: i64) -> UploadAction {
    match existing_attack_count {
        None => UploadAction::Insert,
        Some(existing) if attack_count > existing => UploadAction::Update,
        Some(_) => UploadAction::Skip,
    }
}

fn find_attacks_object(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(attacks)) = map.get("Attacks") {
                return Some(attacks);
            }
            for entry in map.values() {
                if let Some(found) = find_attacks_object(entry) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(find_attacks_object),
        _ => None,
    }
}

fn compress_mail_value(decoded: &Value, zstd_level: i32) -> Result<Vec<u8>, ApiError> {
    let json =
        serde_json::to_vec(decoded).map_err(|error| ApiError::internal(error.to_string()))?;
    zstd::stream::encode_all(Cursor::new(json), zstd_level)
        .map_err(|error| ApiError::internal(error.to_string()))
}

fn decode_lossless_doc(buffer: &[u8]) -> Result<Value, ApiError> {
    let lossless = mail_decoder::decode_lossless(buffer)
        .map_err(|error| ApiError::decode_failed(error.to_string()))?;
    Ok(mail_decoder::lossless_to_json(&lossless))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn extracts_mail_type() {
        let decoded = json!({ "type": "Battle" });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Battle");
    }

    #[test]
    fn extracts_mail_type_from_singleton_array() {
        let decoded = json!([{ "type": "Battle" }]);
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Battle");
    }

    #[test]
    fn supports_known_mail_types() {
        assert!(is_supported_mail_type("Battle"));
        assert!(is_supported_mail_type("DuelBattle2"));
        assert!(is_supported_mail_type("BarCanyonKillBoss"));
        assert!(is_supported_mail_type("Rss"));
        assert!(is_supported_mail_type("SystemBarbarianFort"));
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
    fn extracts_alliance_custom_battle_results_mail_type() {
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
    fn extracts_alliance_custom_individual_results_mail_type() {
        let decoded = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 15
            }
        });
        assert_eq!(
            extract_mail_type(&decoded).unwrap(),
            "AllianceAOOIndividualResults".to_string()
        );
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
    fn extracts_mail_id_from_singleton_array() {
        let decoded = json!([{ "id": "12345" }]);
        assert_eq!(extract_mail_id(&decoded).as_deref(), Some("12345"));
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
    fn counts_attacks_nested() {
        let decoded = json!({
            "body": {
                "content": {
                    "Attacks": {
                        "a": { "id": 1 },
                        "b": { "id": 2 },
                        "c": { "id": 3 }
                    }
                }
            }
        });
        assert_eq!(count_attacks(&decoded), 3);
    }

    #[test]
    fn decide_action_inserts_when_missing() {
        assert!(matches!(decide_action(None, 4), UploadAction::Insert));
    }

    #[test]
    fn decide_action_updates_when_newer() {
        assert!(matches!(decide_action(Some(2), 4), UploadAction::Update));
    }

    #[test]
    fn decide_action_skips_when_not_newer() {
        assert!(matches!(decide_action(Some(5), 4), UploadAction::Skip));
        assert!(matches!(decide_action(Some(4), 4), UploadAction::Skip));
    }

    #[test]
    fn status_mapping_marks_supported_processor_types_processable() {
        assert_eq!(insert_status_for_mail_type("Battle"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("Rss"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("SystemBarbarianFort"), STATUS_PENDING);
        assert_eq!(update_status_for_mail_type("Battle"), STATUS_REPROCESS);
        assert_eq!(update_status_for_mail_type("Rss"), STATUS_REPROCESS);
        assert_eq!(update_status_for_mail_type("SystemBarbarianFort"), STATUS_REPROCESS);
        assert_eq!(insert_status_for_mail_type("AllianceAOOBattleResults"), STATUS_PENDING);
        assert_eq!(update_status_for_mail_type("AllianceAOOBattleResults"), STATUS_REPROCESS);
        assert_eq!(insert_status_for_mail_type("AllianceAOOBattleInfo"), STATUS_PENDING);
        assert_eq!(update_status_for_mail_type("AllianceAOOBattleInfo"), STATUS_REPROCESS);
        assert_eq!(insert_status_for_mail_type("AllianceAOOIndividualResults"), STATUS_PENDING);
        assert_eq!(update_status_for_mail_type("AllianceAOOIndividualResults"), STATUS_REPROCESS);
    }

    #[test]
    fn parses_mail_id_from_filename() {
        let id = parse_mail_id_from_filename("Persistent.Mail.12345").unwrap();
        assert_eq!(id, "12345");
        let id = parse_mail_id_from_filename("Persistent.Mail.999.json").unwrap();
        assert_eq!(id, "999");
    }

    #[test]
    fn rejects_invalid_filename() {
        assert!(parse_mail_id_from_filename("battle.mail.1").is_err());
        assert!(parse_mail_id_from_filename("Persistent.Mail.").is_err());
    }

    #[test]
    fn detects_json_payloads() {
        assert!(is_probably_json(br#"{\"type\":\"Battle\"}"#));
        assert!(is_probably_json(b"  [1,2,3]"));
        assert!(!is_probably_json(b"\xFF\xF5\xDD\x4C"));
    }

    #[test]
    fn validates_content_type() {
        assert!(is_allowed_content_type("application/octet-stream"));
        assert!(!is_allowed_content_type("application/json"));
        assert!(!is_allowed_content_type("text/plain"));
    }

    #[test]
    fn validates_content_encoding() {
        assert!(is_allowed_content_encoding(None));
        assert!(is_allowed_content_encoding(Some(&HeaderValue::from_static("identity"))));
        assert!(!is_allowed_content_encoding(Some(&HeaderValue::from_static("gzip"))));
    }

    #[test]
    fn extract_user_agent_accepts_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("ROKBattles/0.1.0"));
        let user_agent = extract_user_agent(&headers).unwrap();
        assert_eq!(user_agent, "ROKBattles/0.1.0");
    }

    #[test]
    fn extract_user_agent_rejects_missing_header() {
        let headers = HeaderMap::new();
        let err = extract_user_agent(&headers).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn extract_user_agent_rejects_invalid_header() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("OtherApp/0.1.0"));
        let err = extract_user_agent(&headers).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn ua_ok_accepts_minimal_user_agent() {
        assert!(ua_ok("ROKBattles/0.1.0"));
    }

    #[test]
    fn ua_ok_accepts_tauri_suffix() {
        assert!(ua_ok("ROKBattles/0.2.5 (MacOS; Tauri/1.5.0)"));
    }

    #[test]
    fn ua_ok_rejects_missing_prefix() {
        assert!(!ua_ok("OtherApp/0.1.0"));
    }

    #[test]
    fn ua_ok_rejects_missing_version() {
        assert!(!ua_ok("ROKBattles/"));
    }

    #[test]
    fn ua_ok_rejects_suffix_without_tauri() {
        assert!(!ua_ok("ROKBattles/0.1.0 (MacOS; SomethingElse/1.2.3)"));
    }
}
