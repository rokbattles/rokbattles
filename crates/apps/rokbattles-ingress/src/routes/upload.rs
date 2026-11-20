//! Handles direct mail uploads to `POST /v2/upload`.
//!
//! For compressed mail, set `Content-Encoding` on the multipart file part to
//! `gzip` or `br`. For uncompressed mail, omit the header or use `identity`.
//! The request body and the decompressed file each have a 25 MiB limit.

use std::{io::Read, sync::Arc};

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_ENCODING},
    response::IntoResponse,
};
use bytes::Bytes;
use flate2::bufread::MultiGzDecoder;
use rokbattles_mail_registry::is_supported_mail_type;
use serde::Serialize;

use super::{
    MAX_UPLOAD_BYTES,
    headers::{extract_user_agent, is_allowed_content_type},
};
use crate::{
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

/// Validates and stores a mail report.
///
/// Decodes the mail and checks its type and ID before looking up duplicates.
/// Returns `201 Created` for a new report or `200 OK` if the ID already exists.
/// Both responses include the submitted mail ID and type.
pub(super) async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let user_agent = extract_user_agent(&headers)?;
    let upload = read_upload(&mut multipart).await?;
    let buffer = upload.bytes;

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

/// Reads the first file with nonempty mail bytes.
///
/// Checks filenames and headers even for empty files. Rejects JSON-like
/// prefixes after decompression and leaves later parts unread.
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

        let compression = file_compression(field.headers().get(CONTENT_ENCODING))?;
        let bytes =
            field.bytes().await.map_err(|error| ApiError::bad_request(error.to_string()))?;
        let bytes = match compression {
            Some(compression) => tokio::task::spawn_blocking(move || compression.decode(&bytes))
                .await
                .map_err(|error| ApiError::internal(error.to_string()))??,
            None => bytes,
        };

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

fn file_compression(value: Option<&HeaderValue>) -> Result<Option<Compression>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    match value.to_str() {
        Ok(value) if value.eq_ignore_ascii_case("identity") => Ok(None),
        Ok(value) if value.eq_ignore_ascii_case("gzip") => Ok(Some(Compression::Gzip)),
        Ok(value) if value.eq_ignore_ascii_case("br") => Ok(Some(Compression::Brotli)),
        _ => Err(ApiError::bad_request("unsupported content encoding (use identity, gzip, or br)")),
    }
}

enum Compression {
    Gzip,
    Brotli,
}

impl Compression {
    fn decode(self, bytes: &[u8]) -> Result<Bytes, ApiError> {
        match self {
            Self::Gzip => read_decompressed(MultiGzDecoder::new(bytes)),
            Self::Brotli => {
                // Reject the Large Window Brotli marker (RFC 9841, section 6) to
                // keep the decoder's window allocation within the standard limit.
                if bytes.first() == Some(&0x11) {
                    return Err(ApiError::bad_request("unsupported Brotli window format"));
                }
                let mut decoder = brotli::Decompressor::new(bytes, 4096);
                let decoded = read_decompressed(&mut decoder)?;
                // The decoder reports trailing buffered bytes on the read after EOF.
                // Check the underlying slice for any remaining input.
                if !decoder.get_ref().is_empty() || !matches!(decoder.read(&mut [0]), Ok(0)) {
                    return Err(ApiError::bad_request("invalid compressed mail"));
                }
                Ok(decoded)
            }
        }
    }
}

fn read_decompressed(decoder: impl Read) -> Result<Bytes, ApiError> {
    let mut decoded = Vec::new();
    // Read one extra byte to distinguish an oversized file from one at the limit.
    decoder
        .take(MAX_UPLOAD_BYTES as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|_error| ApiError::bad_request("invalid compressed mail"))?;
    if decoded.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::PayloadTooLarge);
    }
    Ok(Bytes::from(decoded))
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
    use flate2::{Compression as GzipLevel, read::GzEncoder};

    use super::*;
    use crate::routes::tests::multipart;

    fn compress_mail(bytes: &[u8]) -> [(&'static str, Vec<u8>); 2] {
        let mut gzip = Vec::new();
        GzEncoder::new(bytes, GzipLevel::default()).read_to_end(&mut gzip).expect("encode gzip");
        let mut br = Vec::new();
        brotli::CompressorReader::new(bytes, 4096, 5, 22)
            .read_to_end(&mut br)
            .expect("encode Brotli");
        [("gzip", gzip), ("br", br)]
    }

    async fn mail_part(encoding: Option<&str>, bytes: &[u8]) -> Multipart {
        let mut body = b"--mail-boundary\r\n\
            Content-Disposition: form-data; name=\"file\"; filename=\"Persistent.Mail.13411379179417835931\"\r\n\
            Content-Type: application/octet-stream\r\n"
            .to_vec();
        if let Some(encoding) = encoding {
            body.extend_from_slice(format!("Content-Encoding: {encoding}\r\n").as_bytes());
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n--mail-boundary--\r\n");
        multipart(body).await
    }

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

    #[tokio::test]
    async fn reads_identical_mail_bytes_for_each_encoding() {
        let raw = include_bytes!(
            "../../../../../samples/GhostCavKillBar/Persistent.Mail.13411379179417835931"
        );
        for encoding in [None, Some("Identity")] {
            let upload = read_upload(&mut mail_part(encoding, raw).await).await.expect("raw mail");
            assert_eq!(upload.bytes.as_ref(), raw);
        }
        for (encoding, bytes) in compress_mail(raw) {
            let upload = read_upload(&mut mail_part(Some(encoding), &bytes).await)
                .await
                .expect("compressed mail");
            assert_eq!(upload.file_id, "13411379179417835931");
            assert_eq!(upload.bytes.as_ref(), raw, "{encoding}");
        }
    }

    #[tokio::test]
    async fn rejects_unsupported_and_stacked_encodings() {
        for encoding in ["zstd", "gzip, br"] {
            let error = read_upload(&mut mail_part(Some(encoding), b"binary-mail").await)
                .await
                .expect_err("unsupported encoding");
            assert!(matches!(
                error,
                ApiError::BadRequest(message) if message.starts_with("unsupported content encoding")
            ));
        }
    }

    #[tokio::test]
    async fn rejects_truncated_streams_and_trailing_bytes() {
        for (encoding, bytes) in compress_mail(b"binary-mail") {
            let truncated = bytes.get(..bytes.len() - 1).expect("truncated compressed stream");
            let mut trailing = bytes.clone();
            trailing.extend_from_slice(b"trailing bytes");
            for invalid in [truncated, trailing.as_slice()] {
                let error = read_upload(&mut mail_part(Some(encoding), invalid).await)
                    .await
                    .expect_err("invalid compressed mail");
                assert!(matches!(
                    error, ApiError::BadRequest(message) if message == "invalid compressed mail"
                ));
            }
        }
    }

    #[tokio::test]
    async fn rejects_json_after_decompression() {
        for (encoding, bytes) in compress_mail(b"{}") {
            let error = read_upload(&mut mail_part(Some(encoding), &bytes).await)
                .await
                .expect_err("JSON payload");
            assert!(matches!(
                error,
                ApiError::BadRequest(message)
                    if message == "expected binary mail buffer, received JSON"
            ));
        }
    }

    #[tokio::test]
    async fn rejects_the_large_window_brotli_extension() {
        let params = brotli::enc::BrotliEncoderParams {
            large_window: true,
            quality: 5,
            ..Default::default()
        };
        let mut bytes = Vec::new();
        brotli::CompressorReader::with_params(b"binary-mail".as_slice(), 4096, &params)
            .read_to_end(&mut bytes)
            .expect("encode large-window Brotli");

        let error = read_upload(&mut mail_part(Some("br"), &bytes).await)
            .await
            .expect_err("large-window Brotli");
        assert!(matches!(
            error, ApiError::BadRequest(message) if message == "unsupported Brotli window format"
        ));
    }

    #[tokio::test]
    async fn enforces_the_decompressed_size_limit() {
        let mut raw = vec![0; MAX_UPLOAD_BYTES];
        for (encoding, bytes) in compress_mail(&raw) {
            let upload = read_upload(&mut mail_part(Some(encoding), &bytes).await)
                .await
                .expect("mail at size limit");
            assert_eq!(upload.bytes.len(), MAX_UPLOAD_BYTES, "{encoding}");
        }
        raw.push(0);
        for (encoding, bytes) in compress_mail(&raw) {
            let error = read_upload(&mut mail_part(Some(encoding), &bytes).await)
                .await
                .expect_err("mail over size limit");
            assert_eq!(error.into_response().status(), StatusCode::PAYLOAD_TOO_LARGE, "{encoding}");
        }
    }
}
