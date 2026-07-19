use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use bytes::Bytes;
use mail_registry::{is_processable_mail_type, is_supported_mail_type};
use mongodb::bson::DateTime;
use serde::Serialize;
use serde_json::Value;

use crate::{
    clamav::{ScanStatus, scan_zstream},
    error::ApiError,
    raw_mail::{self, RawMailDocumentInput},
    state::AppState,
};

const STATUS_PENDING: &str = "pending";
const STATUS_REPROCESS: &str = "reprocess";
const STATUS_UNPROCESSABLE: &str = "unprocessable";

/// Response from the mail upload endpoint.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    status: String,
    mail_id: String,
    mail_type: String,
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

/// Liveness check.
pub async fn health() -> StatusCode {
    StatusCode::OK
}

/// Store a mail report when it is new or newer than the saved copy.
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

    let action =
        store_compressed_raw_mail(&state, &buffer, &decoded, &mail_id, &mail_type, &user_agent)
            .await?;

    let (status, label) = match action {
        UploadAction::Insert => (StatusCode::CREATED, "stored"),
        UploadAction::Update => (StatusCode::OK, "updated"),
        UploadAction::Skip => (StatusCode::OK, "skipped"),
    };

    let response = UploadResponse { status: label.to_string(), mail_id, mail_type };

    Ok((status, Json(response)))
}

async fn store_compressed_raw_mail(
    state: &AppState,
    buffer: &[u8],
    decoded: &Value,
    mail_id: &str,
    mail_type: &str,
    user_agent: &str,
) -> Result<UploadAction, ApiError> {
    let checksum = raw_mail::sha256_hex(buffer);
    let binary_size = i64::try_from(buffer.len())
        .map_err(|_| ApiError::internal("mail binary is too large to store size"))?;
    let existing = state
        .storage
        .find_existing_compressed_raw(mail_id)
        .await
        .map_err(|error| ApiError::database(error.to_string()))?;

    let action = decide_compressed_raw_action(existing.as_ref(), &checksum, buffer.len());

    if matches!(action, UploadAction::Skip) {
        return Ok(action);
    }

    let mail = raw_mail::extract_raw_mail_metadata(decoded)?;
    let status = match action {
        UploadAction::Insert => insert_status_for_mail_type(mail_type),
        UploadAction::Update => update_status_for_mail_type(mail_type),
        UploadAction::Skip => unreachable!("skip returned above"),
    };
    let doc = raw_mail::build_raw_mail_doc(RawMailDocumentInput {
        original_bytes: buffer,
        user_agent,
        checksum: &checksum,
        mail: &mail,
        status,
        now: DateTime::now(),
        zstd_level: state.config.zstd_level,
    })?;

    match action {
        UploadAction::Insert => state
            .storage
            .insert_compressed_raw(doc)
            .await
            .map_err(|error| ApiError::database(error.to_string()))?,
        UploadAction::Update => state
            .storage
            .update_compressed_raw(mail_id, &checksum, binary_size, doc)
            .await
            .map_err(|error| ApiError::database(error.to_string()))?,
        UploadAction::Skip => {}
    }

    Ok(action)
}

/// Acknowledge legacy TCP stream uploads without storing or validating them.
pub async fn upload_tcp_stream() -> StatusCode {
    StatusCode::NO_CONTENT
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
    if let Some(mail_type) = mail_registry::detect_mail_type(decoded) {
        return Ok(mail_type.to_string());
    }
    let raw = mail_registry::raw_mail_type_string(decoded)
        .ok_or_else(|| ApiError::bad_request("missing mail type"))?;
    if mail_registry::MailType::from_label_ignore_ascii_case(&raw).is_some() {
        return Err(ApiError::unsupported_type(raw));
    }
    Ok(raw)
}

fn insert_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_PENDING } else { STATUS_UNPROCESSABLE }
}

fn update_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_REPROCESS } else { STATUS_UNPROCESSABLE }
}

fn extract_mail_id(decoded: &Value) -> Option<String> {
    let root = mail_registry::normalize_mail_root(decoded)?;
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

/// Read and validate the user agent header.
///
/// Expected format: `ROKBattles/<version>`, with an optional `Tauri/` suffix.
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

/// Check the `ROKBattles/<version>` prefix and optional Tauri suffix.
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

fn decide_compressed_raw_action(
    existing: Option<&crate::storage::ExistingCompressedRawMail>,
    checksum: &str,
    size: usize,
) -> UploadAction {
    match existing {
        None => UploadAction::Insert,
        Some(existing) if existing.checksum.as_deref() == Some(checksum) => UploadAction::Skip,
        Some(existing) if existing.size.is_some_and(|existing_size| size > existing_size) => {
            UploadAction::Update
        }
        Some(_) => UploadAction::Skip,
    }
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
    fn rejects_mail_type_from_singleton_array() {
        let decoded = json!([{ "type": "Battle" }]);
        assert!(extract_mail_type(&decoded).is_err());
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

    fn existing_compressed_raw(
        checksum: &str,
        size: Option<usize>,
    ) -> crate::storage::ExistingCompressedRawMail {
        crate::storage::ExistingCompressedRawMail { checksum: Some(checksum.to_string()), size }
    }

    #[test]
    fn compressed_raw_action_inserts_when_missing() {
        let action = decide_compressed_raw_action(None, "new", 100);
        assert!(matches!(action, UploadAction::Insert));
    }

    #[test]
    fn compressed_raw_action_skips_matching_checksum() {
        let existing = existing_compressed_raw("same", Some(50));
        let action = decide_compressed_raw_action(Some(&existing), "same", 100);
        assert!(matches!(action, UploadAction::Skip));
    }

    #[test]
    fn compressed_raw_action_updates_different_larger_binary() {
        let existing = existing_compressed_raw("old", Some(99));
        let action = decide_compressed_raw_action(Some(&existing), "new", 100);
        assert!(matches!(action, UploadAction::Update));
    }

    #[test]
    fn compressed_raw_action_skips_different_equal_size_binary() {
        let existing = existing_compressed_raw("old", Some(100));
        let action = decide_compressed_raw_action(Some(&existing), "new", 100);
        assert!(matches!(action, UploadAction::Skip));
    }

    #[test]
    fn compressed_raw_action_skips_different_smaller_binary() {
        let existing = existing_compressed_raw("old", Some(101));
        let action = decide_compressed_raw_action(Some(&existing), "new", 100);
        assert!(matches!(action, UploadAction::Skip));
    }

    #[test]
    fn compressed_raw_action_skips_when_stored_size_is_missing() {
        let existing = existing_compressed_raw("old", None);
        let action = decide_compressed_raw_action(Some(&existing), "new", 100);
        assert!(matches!(action, UploadAction::Skip));
    }

    #[test]
    fn status_mapping_marks_supported_processor_types_processable() {
        assert_eq!(insert_status_for_mail_type("Battle"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("Rss"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("SystemBarbarianFort"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("SystemKaharTreasure"), STATUS_PENDING);
        assert_eq!(update_status_for_mail_type("Battle"), STATUS_REPROCESS);
        assert_eq!(update_status_for_mail_type("Rss"), STATUS_REPROCESS);
        assert_eq!(update_status_for_mail_type("SystemBarbarianFort"), STATUS_REPROCESS);
        assert_eq!(update_status_for_mail_type("SystemKaharTreasure"), STATUS_REPROCESS);
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

    #[tokio::test]
    async fn upload_tcp_stream_returns_no_content() {
        let status = upload_tcp_stream().await;

        assert_eq!(status, StatusCode::NO_CONTENT);
    }
}
