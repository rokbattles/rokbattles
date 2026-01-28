use std::io::Cursor;
use std::sync::Arc;

use axum::Json;
use axum::extract::Multipart;
use axum::extract::State;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use bytes::Bytes;
use mongodb::bson::{Binary, Bson, DateTime, doc, spec::BinarySubtype};
use serde::Serialize;
use serde_json::Value;

use crate::clamav::{ScanStatus, scan_zstream};
use crate::error::ApiError;
use crate::state::AppState;

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
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let upload = read_upload(&mut multipart).await?;
    let buffer = upload.bytes;

    if state.config.clamav_enabled {
        let timeout = std::time::Duration::from_millis(state.config.clamav_timeout_ms);
        match scan_zstream(&buffer, &state.config.clamav_addr, timeout).await {
            Ok(ScanStatus::Clean) => {}
            Ok(ScanStatus::Infected(reason)) => {
                return Err(ApiError::bad_request(format!(
                    "clamav detected malware: {reason}"
                )));
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

    let action = decide_action(
        existing.as_ref().map(|entry| entry.attack_count),
        attack_count,
    );

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
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
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
            let bytes = field
                .bytes()
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            if !bytes.is_empty() {
                if is_probably_json(&bytes) {
                    return Err(ApiError::bad_request(
                        "expected binary mail buffer, received JSON",
                    ));
                }
                return Ok(UploadInput {
                    bytes,
                    file_name,
                    file_id,
                });
            }
        }
    }

    Err(ApiError::bad_request("missing upload file"))
}

fn extract_mail_type(decoded: &Value) -> Result<String, ApiError> {
    decoded
        .get("type")
        .and_then(value_to_string)
        .ok_or_else(|| ApiError::bad_request("missing mail type"))
}

fn is_supported_mail_type(mail_type: &str) -> bool {
    matches!(mail_type, "Battle" | "DuelBattle2" | "BarCanyonKillBoss")
}

fn extract_mail_id(decoded: &Value) -> Option<String> {
    decoded
        .get("id")
        .and_then(value_to_string)
        .or_else(|| decoded.get("mail_id").and_then(value_to_string))
        .or_else(|| {
            decoded
                .get("metadata")
                .and_then(|meta| meta.get("mail_id"))
                .and_then(value_to_string)
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
        return Err(ApiError::bad_request(
            "filename must include numeric mail id",
        ));
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
    value
        .to_str()
        .map(|value| value.eq_ignore_ascii_case("identity"))
        .unwrap_or(false)
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
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_mail_type() {
        let decoded = json!({ "type": "Battle" });
        assert_eq!(extract_mail_type(&decoded).unwrap(), "Battle");
    }

    #[test]
    fn supports_known_mail_types() {
        assert!(is_supported_mail_type("Battle"));
        assert!(is_supported_mail_type("DuelBattle2"));
        assert!(is_supported_mail_type("BarCanyonKillBoss"));
        assert!(!is_supported_mail_type("Unknown"));
    }

    #[test]
    fn extracts_mail_id_from_id() {
        let decoded = json!({ "id": "12345" });
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
        assert!(is_allowed_content_encoding(Some(
            &HeaderValue::from_static("identity")
        )));
        assert!(!is_allowed_content_encoding(Some(
            &HeaderValue::from_static("gzip")
        )));
    }
}
