//! Direct binary uploads: validate the request, scan if configured, then store once.

use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use bytes::Bytes;
use rokbattles_mail_registry::is_supported_mail_type;
use serde::Serialize;

use super::headers::{extract_user_agent, is_allowed_content_type};
use crate::{
    clamav::{ScanStatus, scan_instream},
    error::ApiError,
    mail::{
        self, StoreOutcome,
        metadata::{extract_mail_id, extract_mail_type},
    },
    state::AppState,
};

/// Response from the mail upload endpoint.
#[derive(Debug, Serialize)]
struct UploadResponse {
    status: &'static str,
    mail_id: String,
    mail_type: String,
}

#[derive(Debug)]
struct UploadInput {
    bytes: Bytes,
    file_name: String,
    file_id: String,
}

/// Stores a new mail report, skipping any previously uploaded mail ID.
///
/// Scanning, decoding, and type and ID validation run before duplicate lookup.
/// Returns `201` for a new mail or `200` for a skip; both include the submitted
/// mail's ID and type, not metadata read back from storage.
pub(super) async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let user_agent = extract_user_agent(&headers)?;
    let upload = read_upload(&mut multipart).await?;
    let buffer = upload.bytes;

    if let Some(addr) = state.config.clamav_addr.as_deref() {
        match scan_instream(&buffer, addr, Duration::from_secs(15)).await {
            Ok(ScanStatus::Clean) => {}
            Ok(ScanStatus::Infected(reason)) => {
                return Err(ApiError::bad_request(format!("clamav detected malware: {reason}")));
            }
            Err(error) => {
                return Err(ApiError::clamav(error.to_string()));
            }
        }
    }

    let decoded = rokbattles_mail_codec::decode(&buffer)
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
        mail::store(&state.storage, &buffer, &decoded, &mail_id, &mail_type, user_agent).await?;

    let status = match action {
        StoreOutcome::Stored => StatusCode::CREATED,
        StoreOutcome::Skipped => StatusCode::OK,
    };

    let response = UploadResponse { status: action.label(), mail_id, mail_type };

    Ok((status, Json(response)))
}

/// Reads the first nonempty file, rejecting invalid headers or JSON-like data.
///
/// Empty files are skipped only after validating their filename and headers.
/// Returns as soon as a file is selected; later parts are not read.
async fn read_upload(multipart: &mut Multipart) -> Result<UploadInput, ApiError> {
    while let Some(field) =
        multipart.next_field().await.map_err(|error| ApiError::bad_request(error.to_string()))?
    {
        if field.file_name().is_none() && field.name().is_none() {
            continue;
        }

        let file_name = field
            .file_name()
            .map(|name| name.to_string())
            .ok_or_else(|| ApiError::bad_request("missing file name"))?;
        let file_id = parse_mail_id_from_filename(&file_name)?;

        let content_type =
            field.content_type().ok_or_else(|| ApiError::bad_request("missing content type"))?;
        if !is_allowed_content_type(content_type) {
            return Err(ApiError::bad_request(format!("unsupported content type: {content_type}")));
        }

        if !is_allowed_content_encoding(field.headers().get("content-encoding")) {
            return Err(ApiError::bad_request("unsupported content encoding (must be identity)"));
        }

        let bytes =
            field.bytes().await.map_err(|error| ApiError::bad_request(error.to_string()))?;

        if bytes.is_empty() {
            continue;
        }

        if is_probably_json(&bytes) {
            return Err(ApiError::bad_request("expected binary mail buffer, received JSON"));
        }

        return Ok(UploadInput { bytes, file_name, file_id });
    }

    Err(ApiError::bad_request("missing upload file"))
}

/// Extracts consecutive digits after `Persistent.Mail.` in the basename.
/// Any suffix after the ID is ignored.
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

fn is_allowed_content_encoding(value: Option<&HeaderValue>) -> bool {
    let Some(value) = value else {
        return true;
    };

    value.to_str().map(|value| value.eq_ignore_ascii_case("identity")).unwrap_or(false)
}

/// Detects a JSON-like prefix for an early error; `false` does not validate the binary.
fn is_probably_json(bytes: &[u8]) -> bool {
    let sample = bytes.get(..256).unwrap_or(bytes);
    let Ok(text) = std::str::from_utf8(sample) else {
        return false;
    };

    let trimmed = text.trim_start_matches(|ch: char| ch.is_whitespace() || ch == '\u{feff}');
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::tests::multipart;

    #[tokio::test]
    async fn reads_first_nonempty_upload_file() {
        let mut multipart = multipart(concat!(
            "--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"Persistent.Mail.1\"\r\n",
            "Content-Type: application/octet-stream\r\n\r\n",
            "\r\n--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"Persistent.Mail.2\"\r\n",
            "Content-Type: application/octet-stream\r\n\r\n",
            "binary-mail\r\n--mail-boundary--\r\n",
        ))
        .await;

        let upload = read_upload(&mut multipart).await.expect("read nonempty file");

        assert_eq!(upload.file_id, "2");
        assert_eq!(upload.file_name, "Persistent.Mail.2");
        assert_eq!(upload.bytes.as_ref(), b"binary-mail");
    }

    #[tokio::test]
    async fn rejects_json_disguised_as_binary_upload() {
        let mut multipart = multipart(concat!(
            "--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"Persistent.Mail.1\"\r\n",
            "Content-Type: application/octet-stream\r\n\r\n",
            "{}\r\n--mail-boundary--\r\n",
        ))
        .await;

        assert!(matches!(
            read_upload(&mut multipart).await,
            Err(ApiError::BadRequest(message))
                if message == "expected binary mail buffer, received JSON"
        ));
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
        parse_mail_id_from_filename("battle.mail.1").expect_err("input should be rejected");
        parse_mail_id_from_filename("Persistent.Mail.").expect_err("input should be rejected");
    }

    #[test]
    fn detects_json_payloads() {
        assert!(is_probably_json(br#"{\"type\":\"Battle\"}"#));
        assert!(is_probably_json(b"  [1,2,3]"));
        assert!(!is_probably_json(b"\xFF\xF5\xDD\x4C"));
    }

    #[test]
    fn validates_content_encoding() {
        assert!(is_allowed_content_encoding(None));
        assert!(is_allowed_content_encoding(Some(&HeaderValue::from_static("identity"))));
        assert!(!is_allowed_content_encoding(Some(&HeaderValue::from_static("gzip"))));
    }
}
