use std::{io::Cursor, sync::Arc};

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use bytes::Bytes;
use core_tcp_stream::{TcpStreamBatch, ValidatedTcpStreamBatch};
use mail_registry::{is_processable_mail_type, is_supported_mail_type};
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

/// Response from the mail upload endpoint.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    status: String,
    mail_id: String,
    mail_type: String,
    mail_attack_count: i64,
}

/// Response from the TCP stream upload endpoint.
#[derive(Debug, Serialize)]
pub struct TcpStreamUploadResponse {
    status: String,
    capture_id: String,
    fragment_count: usize,
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

/// Store one TCP stream fragment batch for later processing.
pub async fn upload_tcp_stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(batch): Json<TcpStreamBatch>,
) -> Result<impl IntoResponse, ApiError> {
    let user_agent = extract_user_agent(&headers)?;
    let validated = batch.validate().map_err(|error| ApiError::bad_request(error.to_string()))?;
    let fragment_count = validated.fragments.len();
    let capture_id = validated.capture_id.clone();

    let doc = tcp_stream_doc(&validated, &user_agent, DateTime::now())?;
    let batch_index = doc
        .get_i64("batch_index")
        .map_err(|_| ApiError::internal("missing tcp stream batch index"))?;
    state
        .storage
        .upsert_tcp_stream_raw(&capture_id, batch_index, doc)
        .await
        .map_err(|error| ApiError::database(error.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(TcpStreamUploadResponse { status: "stored".to_string(), capture_id, fragment_count }),
    ))
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
    mail_registry::raw_mail_type_string(decoded)
        .ok_or_else(|| ApiError::bad_request("missing mail type"))
}

fn insert_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_PENDING } else { STATUS_UNPROCESSABLE }
}

fn update_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_REPROCESS } else { STATUS_UNPROCESSABLE }
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

/// Turn a decoded mail payload into the root object we store.
///
/// Some mail samples decode as a one-item array. In that case the item is the
/// root object. Other shapes are rejected.
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

fn tcp_stream_doc(
    batch: &ValidatedTcpStreamBatch,
    user_agent: &str,
    now: DateTime,
) -> Result<mongodb::bson::Document, ApiError> {
    let fragments = batch
        .fragments
        .iter()
        .map(|fragment| {
            Ok(Bson::Document(doc! {
                "index": u64_to_i64(fragment.index, "fragment index")?,
                "direction": match fragment.direction {
                    core_tcp_stream::Direction::ClientToServer => "client_to_server",
                    core_tcp_stream::Direction::ServerToClient => "server_to_client",
                },
                "payload": Bson::Binary(Binary {
                    subtype: BinarySubtype::Generic,
                    bytes: fragment.payload.clone(),
                }),
            }))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(doc! {
        "capture_id": &batch.capture_id,
        "batch_index": u64_to_i64(batch.batch_index, "batch index")?,
        "stream_ended": batch.stream_ended,
        "status": STATUS_PENDING,
        "user_agent": user_agent,
        "stream": {
            "client_addr": batch.stream.client_addr.to_string(),
            "client_port": i32::from(batch.stream.client_port),
            "server_addr": batch.stream.server_addr.to_string(),
            "server_port": i32::from(batch.stream.server_port),
        },
        "handshake": {
            "api_id": u64_to_i64(batch.handshake.api_id, "handshake api id")?,
            "key1": u64_to_i64(batch.handshake.key1, "handshake key1")?,
            "key2": u64_to_i64(batch.handshake.key2, "handshake key2")?,
        },
        "fragment_count": i32::try_from(batch.fragments.len())
            .map_err(|_| ApiError::bad_request("too many tcp fragments"))?,
        "fragments": Bson::Array(fragments),
        "createdAt": now,
        "updatedAt": now,
    })
}

fn u64_to_i64(value: u64, label: &str) -> Result<i64, ApiError> {
    i64::try_from(value).map_err(|_| ApiError::bad_request(format!("{label} is too large")))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use core_tcp_stream::{CLIENT_PORT, Direction, Handshake, StreamId, TcpStreamFragmentUpload};
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

    #[test]
    fn tcp_stream_doc_stores_fragment_payloads_as_binary() {
        let batch = TcpStreamBatch {
            capture_id: "capture-1".to_string(),
            batch_index: 0,
            stream_ended: false,
            stream: StreamId {
                client_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
                client_port: 56_380,
                server_addr: IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)),
                server_port: CLIENT_PORT,
            },
            handshake: Handshake { api_id: 8562, key1: 1, key2: 2 },
            fragments: vec![TcpStreamFragmentUpload::from_payload(
                0,
                Direction::ServerToClient,
                &[0x00, 0x02, 0xaa, 0xbb],
            )],
        };
        let validated = batch.validate().expect("batch should validate");

        let doc = tcp_stream_doc(&validated, "ROKBattles/1.0.0", DateTime::now())
            .expect("doc should build");

        assert_eq!(doc.get_i32("fragment_count"), Ok(1));
    }
}
