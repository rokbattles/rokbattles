use crate::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use blake3::Hasher;
use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::Document,
    bson::spec::BinarySubtype,
    bson::{Binary, Bson, DateTime, doc},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, info, warn};
use zstd::encode_all;

const MAX_UPLOAD: usize = 10 * 1024 * 1024; // 10 MB
const BUFFER_LEN: usize = 32;
const SUPPORTED_MINOR_VERSIONS: &[u64] = &[2, 3];

fn ua_ok(h: &HeaderMap) -> bool {
    let Some(user_agent) = h.get("user-agent").and_then(|v| v.to_str().ok()) else {
        return false;
    };

    let Some(rest) = user_agent.strip_prefix("ROKBattles/") else {
        return false;
    };

    let mut parts = rest.splitn(2, ' ');
    let version = parts.next().unwrap_or_default();
    if !is_supported_version(version) {
        return false;
    }

    match parts.next() {
        None => true,
        Some(remainder) => remainder.starts_with('(') && remainder.contains(" Tauri/"),
    }
}

fn is_supported_version(version: &str) -> bool {
    let mut segments = version.split('.');
    let (Some(major), Some(minor), Some(patch)) =
        (segments.next(), segments.next(), segments.next())
    else {
        return false;
    };

    if segments.next().is_some() {
        return false;
    }

    if !(is_numeric(major) && is_numeric(minor) && is_numeric(patch)) {
        return false;
    }

    matches!((major.parse::<u64>(), minor.parse::<u64>()), (Ok(0), Ok(minor)) if SUPPORTED_MINOR_VERSIONS.contains(&minor))
}

fn is_numeric(input: &str) -> bool {
    !input.is_empty() && input.chars().all(|c| c.is_ascii_digit())
}

fn ct_ok(h: &HeaderMap) -> bool {
    matches!(
        h.get("content-type").and_then(|v| v.to_str().ok()),
        Some(ct) if ct.starts_with("application/octet-stream")
    )
}

fn ce_ok(h: &HeaderMap) -> bool {
    match h.get("content-encoding").and_then(|v| v.to_str().ok()) {
        None => true,
        Some(enc) => enc.eq_ignore_ascii_case("identity"),
    }
}

fn cl_ok(h: &HeaderMap) -> Result<usize, (StatusCode, &'static str)> {
    let len = h
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or((StatusCode::LENGTH_REQUIRED, "missing content-length"))?;
    if len == 0 || len > MAX_UPLOAD {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "content-length too large"));
    }
    Ok(len)
}

async fn clamd_begin(addr: &str) -> Result<TcpStream, (StatusCode, &'static str)> {
    let mut s = TcpStream::connect(addr)
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "service unavailable"))?;
    s.write_all(b"zINSTREAM\0")
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "service error"))?;
    Ok(s)
}

async fn clamd_send_chunk(
    s: &mut TcpStream,
    chunk: &[u8],
) -> Result<(), (StatusCode, &'static str)> {
    if chunk.is_empty() {
        return Ok(());
    }
    let len = (chunk.len() as u32).to_be_bytes();
    s.write_all(&len)
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "service error"))?;
    s.write_all(chunk)
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "service error"))?;
    Ok(())
}

async fn clamd_end(mut s: TcpStream) -> Result<(), (StatusCode, &'static str)> {
    s.write_all(&0u32.to_be_bytes())
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "service error"))?;
    let mut resp = Vec::new();
    s.read_to_end(&mut resp).await.ok();
    let verdict = String::from_utf8_lossy(&resp);
    if verdict.contains("FOUND") {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "service rejected entity"));
    }
    Ok(())
}

fn zstd_compress(s: &str, level: i32) -> anyhow::Result<Vec<u8>> {
    Ok(encode_all(s.as_bytes(), level)?)
}

fn blake3_hash(data: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hash.to_hex().to_string()
}

pub async fn ingress(State(st): State<AppState>, req: Request<Body>) -> impl IntoResponse {
    let headers = req.headers().clone();

    if !ua_ok(&headers) {
        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        debug!(user_agent, "ingress rejected: unsupported user agent");
        return (StatusCode::FORBIDDEN, "bad user agent").into_response();
    }

    if !ct_ok(&headers) {
        debug!(
            content_type = headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown"),
            "ingress rejected: unsupported content type"
        );
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "bad content type").into_response();
    }

    if !ce_ok(&headers) {
        debug!(
            content_encoding = headers
                .get("content-encoding")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown"),
            "ingress rejected: unsupported content encoding"
        );
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "bad content encoding").into_response();
    }

    let content_len = match cl_ok(&headers) {
        Ok(n) => {
            debug!(content_length = n, "validated content length");
            n
        }
        Err((status, reason)) => {
            debug!(status = %status, reason, "ingress rejected: invalid content length");
            return (status, reason).into_response();
        }
    };

    let mut total = 0usize;
    let mut peek = Vec::with_capacity(BUFFER_LEN);
    let mut remaining: Option<Vec<u8>> = None;
    let mut buf = Vec::with_capacity(content_len.min(MAX_UPLOAD));

    let mut stream = req.into_body().into_data_stream();

    while peek.len() < BUFFER_LEN {
        let Some(next) = stream.next().await else {
            break;
        };
        let chunk = match next {
            Ok(bytes) => bytes,
            Err(_) => return (StatusCode::BAD_REQUEST, "bad request").into_response(),
        };
        if chunk.is_empty() {
            continue;
        }
        total += chunk.len();
        if total > MAX_UPLOAD {
            debug!(
                total_bytes = total,
                "ingress rejected: payload exceeded limit during peek"
            );
            return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response();
        }

        let rem = BUFFER_LEN - peek.len();
        if chunk.len() <= rem {
            peek.extend_from_slice(&chunk);
            buf.extend_from_slice(&chunk);
        } else {
            peek.extend_from_slice(&chunk[..rem]);
            buf.extend_from_slice(&chunk[..rem]);
            remaining = Some(chunk[rem..].to_vec());
            break;
        }
    }

    if !mail_decoder::has_rok_mail_header(&peek) {
        debug!("ingress rejected: missing rok mail header");
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "not rok mail").into_response();
    }

    let mut clamd = match clamd_begin(&st.clamd_addr).await {
        Ok(s) => s,
        Err((status, reason)) => {
            warn!(status = %status, reason, "clamd connection failed");
            return (status, reason).into_response();
        }
    };

    if let Err((status, reason)) = clamd_send_chunk(&mut clamd, &peek).await {
        warn!(status = %status, reason, "clamd send chunk failed during peek");
        return (status, reason).into_response();
    }
    if let Some(r) = remaining.take() {
        if let Err((status, reason)) = clamd_send_chunk(&mut clamd, &r).await {
            warn!(status = %status, reason, "clamd send chunk failed after peek");
            return (status, reason).into_response();
        }
        buf.extend_from_slice(&r);
    }

    loop {
        match stream.try_next().await {
            Ok(Some(chunk)) => {
                if chunk.is_empty() {
                    continue;
                }
                total += chunk.len();
                if total > MAX_UPLOAD {
                    debug!(
                        total_bytes = total,
                        "ingress rejected: payload exceeded limit during stream"
                    );
                    return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response();
                }
                if let Err((status, reason)) = clamd_send_chunk(&mut clamd, &chunk).await {
                    warn!(status = %status, reason, "clamd send chunk failed during stream");
                    return (status, reason).into_response();
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => {
                debug!("ingress rejected: body stream error");
                return (StatusCode::BAD_REQUEST, "bad request").into_response();
            }
        }
    }

    if total != content_len {
        debug!(
            total_bytes = total,
            content_length = content_len,
            "ingress rejected: content length mismatch"
        );
        return (StatusCode::BAD_REQUEST, "bad request").into_response();
    }

    if let Err((status, reason)) = clamd_end(clamd).await {
        warn!(status = %status, reason, "clamd did not accept stream");
        return (status, reason).into_response();
    }

    let decoded_mail = match mail_decoder::decode(&buf).map(|m| m.into_owned()) {
        Ok(m) => m,
        Err(_) => {
            warn!("ingress rejected: mail decoder failed");
            return (StatusCode::UNPROCESSABLE_ENTITY, "rok mail decoder failed").into_response();
        }
    };

    let mail_type = mail_helper::detect_mail_type_str(&decoded_mail);
    if !mail_type.is_some_and(|t| t.eq_ignore_ascii_case("Battle")) {
        debug!("ingress rejected: not a battle mail");
        return (StatusCode::UNPROCESSABLE_ENTITY, "not a rok battle mail").into_response();
    }

    let mail_time = match mail_helper::detect_mail_time(&decoded_mail) {
        Some(t) => t,
        None => {
            debug!("ingress rejected: mail missing time");
            return (StatusCode::UNPROCESSABLE_ENTITY, "rok mail missing time").into_response();
        }
    };
    let mail_id = match mail_helper::detect_email_id(&decoded_mail) {
        Some(id) => id,
        None => {
            debug!("ingress rejected: mail missing id");
            return (StatusCode::UNPROCESSABLE_ENTITY, "rok mail missing id").into_response();
        }
    };

    let decoded_mail_hash = blake3_hash(&buf);
    debug!(decoded_mail_hash = %decoded_mail_hash, "computed decoded mail hash");
    debug!(
        mail_id = %mail_id,
        mail_time = ?mail_time,
        mail_type = mail_type.unwrap_or("unknown"),
        "decoded mail metadata"
    );

    let decoded_mail_json = serde_json::to_value(&decoded_mail).unwrap();
    let decoded_mail_json_text = serde_json::to_string(&decoded_mail_json).unwrap();

    let compressed_mail = match zstd_compress(&decoded_mail_json_text, 5) {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "compression failed").into_response(),
    };

    let compression_ratio = (compressed_mail.len() as f64) / (decoded_mail_json_text.len() as f64);
    debug!(
        original_bytes = decoded_mail_json_text.len(),
        compressed_bytes = compressed_mail.len(),
        compression_ratio,
        compression_ratio_pct = compression_ratio * 100.0,
        "compressed decoded mail"
    );

    let original_size = decoded_mail_json_text.len() as i64;
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let mails_collection = st.db.collection::<Document>("mails");
    let existing_mail = match mails_collection
        .find_one(doc! { "mail.id": &mail_id })
        .await
    {
        Ok(doc) => doc,
        Err(err) => {
            warn!(error = %err, "database lookup for existing mail failed");
            return (StatusCode::SERVICE_UNAVAILABLE, "service error").into_response();
        }
    };

    if let Some(existing_doc) = existing_mail {
        let existing_size = existing_doc
            .get_document("metadata")
            .ok()
            .and_then(|metadata| metadata.get("originalSize"))
            .and_then(|value| match value {
                Bson::Int32(v) => Some(i64::from(*v)),
                Bson::Int64(v) => Some(*v),
                Bson::Double(v) => Some(*v as i64),
                _ => None,
            })
            .unwrap_or(0);

        let mut previous_hashes = existing_doc
            .get_document("metadata")
            .ok()
            .and_then(|metadata| metadata.get("previousHashes"))
            .and_then(|value| match value {
                Bson::Array(values) => Some(
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(|v| v.to_string()))
                        .collect::<Vec<String>>(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        let previous_hash = existing_doc
            .get_document("mail")
            .ok()
            .and_then(|mail| mail.get_str("hash").ok())
            .map(|hash| hash.to_string());

        let will_update = original_size > existing_size;
        debug!(
            mail_id = %mail_id,
            existing_size,
            original_size,
            will_update,
            "found existing mail"
        );

        if original_size > existing_size {
            let mut update = doc! {
                "mail.hash": &decoded_mail_hash,
                "mail.value": Binary { subtype: BinarySubtype::Generic, bytes: compressed_mail.clone() },
                "mail.time": mail_time,
                "mail.id": &mail_id,
                "metadata.userAgent": user_agent,
                "metadata.originalSize": original_size,
                "status": "reprocess"
            };

            if let Some(prev_hash) = &previous_hash
                && !previous_hashes.iter().any(|hash| hash == prev_hash)
            {
                previous_hashes.push(prev_hash.clone());
            }

            if !previous_hashes.is_empty() {
                let bson_previous_hashes = previous_hashes
                    .iter()
                    .cloned()
                    .map(Bson::String)
                    .collect::<Vec<Bson>>();
                update.insert("metadata.previousHashes", Bson::Array(bson_previous_hashes));
            }

            let result = mails_collection
                .update_one(doc! { "mail.id": &mail_id }, doc! { "$set": update })
                .await;

            return match result {
                Ok(_) => {
                    info!(
                        mail_id = %mail_id,
                        previous_size = existing_size,
                        new_size = original_size,
                        "stored updated mail version"
                    );
                    (StatusCode::CREATED, "").into_response()
                }
                Err(err) => {
                    warn!(mail_id = %mail_id, error = %err, "database update failed");
                    (StatusCode::SERVICE_UNAVAILABLE, "service error").into_response()
                }
            };
        }

        debug!(
            mail_id = %mail_id,
            existing_size,
            original_size,
            "skipping update because incoming mail is not larger"
        );
        return (StatusCode::CREATED, "").into_response();
    }

    let current_time = DateTime::now();
    let doc = doc! {
        "mail": {
            "hash": &decoded_mail_hash,
            "codec": "zstd",
            "value": Binary { subtype: BinarySubtype::Generic, bytes: compressed_mail },
            "time": mail_time,
            "id": &mail_id
        },
        "metadata": {
            "userAgent": user_agent,
            "originalSize": original_size,
        },
        "status": "pending",
        "createdAt": current_time,
    };

    debug!(mail_id = %mail_id, "upserting new mail document");
    let result = mails_collection
        .update_one(doc! { "mail.id": &mail_id }, doc! { "$setOnInsert": doc })
        .upsert(true)
        .await;

    match result {
        Ok(_) => {
            info!(mail_id = %mail_id, "stored new mail document");
            (StatusCode::CREATED, "").into_response()
        }
        Err(err) => {
            warn!(mail_id = %mail_id, error = %err, "database upsert failed");
            (StatusCode::SERVICE_UNAVAILABLE, "service error").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_supported_version, ua_ok};
    use axum::http::HeaderMap;

    fn headers_with_user_agent(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", value.parse().unwrap());
        headers
    }

    #[test]
    fn ua_ok_rejects_supported_minimal_user_agent() {
        let headers = headers_with_user_agent("ROKBattles/0.1.0");
        assert!(!ua_ok(&headers));
    }

    #[test]
    fn ua_ok_accepts_supported_tauri_user_agent() {
        let headers = headers_with_user_agent("ROKBattles/0.2.5 (MacOS; Tauri/1.5.0)");
        assert!(ua_ok(&headers));
    }

    #[test]
    fn ua_ok_rejects_missing_prefix() {
        let headers = headers_with_user_agent("OtherApp/0.1.0");
        assert!(!ua_ok(&headers));
    }

    #[test]
    fn ua_ok_rejects_missing_user_agent_header() {
        let headers = HeaderMap::new();
        assert!(!ua_ok(&headers));
    }

    #[test]
    fn ua_ok_rejects_unsupported_minor_version() {
        let headers = headers_with_user_agent("ROKBattles/0.30.0");
        assert!(!ua_ok(&headers));
    }

    #[test]
    fn ua_ok_rejects_invalid_suffix() {
        let headers = headers_with_user_agent("ROKBattles/0.1.0 missing-tauri");
        assert!(!ua_ok(&headers));
    }

    #[test]
    fn ua_ok_rejects_suffix_without_tauri_identifier() {
        let headers = headers_with_user_agent("ROKBattles/0.1.0 (MacOS; SomethingElse/1.2.3)");
        assert!(!ua_ok(&headers));
    }

    #[test]
    fn is_supported_version_accepts_allowed_minor_versions() {
        assert!(is_supported_version("0.3.0"));
        assert!(is_supported_version("0.2.10"));
    }

    #[test]
    fn is_supported_version_rejects_invalid_formats() {
        for version in ["0.1", "0.1.0.1", "0.a.1", "", "0.1."] {
            assert!(!is_supported_version(version));
        }
    }

    #[test]
    fn is_supported_version_rejects_disallowed_major_or_minor() {
        assert!(!is_supported_version("1.0.0"));
        assert!(!is_supported_version("0.30.0"));
    }
}
