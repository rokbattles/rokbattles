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
use tracing::debug;
use zstd::encode_all;

const MAX_UPLOAD: usize = 5 * 1024 * 1024; // 5 MB
const BUFFER_LEN: usize = 32;

// TODO we'll be changing this to load supported versions at runtime, we'll stop supporting older versions over time
fn ua_ok(h: &HeaderMap) -> bool {
    h.get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("ROKBattles/0.1.0"))
        .unwrap_or(false)
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
        return (StatusCode::FORBIDDEN, "bad user agent").into_response();
    }

    if !ct_ok(&headers) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "bad content type").into_response();
    }

    if !ce_ok(&headers) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "bad content encoding").into_response();
    }

    let content_len = match cl_ok(&headers) {
        Ok(n) => n,
        Err(e) => return e.into_response(),
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
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "not rok mail").into_response();
    }

    let mut clamd = match clamd_begin(&st.clamd_addr).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };

    if let Err(e) = clamd_send_chunk(&mut clamd, &peek).await {
        return e.into_response();
    }
    if let Some(r) = remaining.take() {
        if let Err(e) = clamd_send_chunk(&mut clamd, &r).await {
            return e.into_response();
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
                    return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response();
                }
                if let Err(e) = clamd_send_chunk(&mut clamd, &chunk).await {
                    return e.into_response();
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "bad request").into_response();
            }
        }
    }

    if total != content_len {
        return (StatusCode::BAD_REQUEST, "bad request").into_response();
    }

    if let Err(e) = clamd_end(clamd).await {
        return e.into_response();
    }

    let decoded_mail = match mail_decoder::decode(&buf) {
        Ok(m) => m,
        Err(_) => {
            return (StatusCode::UNPROCESSABLE_ENTITY, "rok mail decoder failed").into_response();
        }
    };

    let mail_type = mail_helper::detect_mail_type_str(&decoded_mail);
    if !mail_type.is_some_and(|t| t.eq_ignore_ascii_case("Battle")) {
        return (StatusCode::UNPROCESSABLE_ENTITY, "not a rok battle mail").into_response();
    }

    let mail_time = match mail_helper::detect_mail_time(&decoded_mail) {
        Some(t) => t,
        None => return (StatusCode::UNPROCESSABLE_ENTITY, "rok mail missing time").into_response(),
    };
    let mail_id = match mail_helper::detect_email_id(&decoded_mail) {
        Some(id) => id,
        None => return (StatusCode::UNPROCESSABLE_ENTITY, "rok mail missing id").into_response(),
    };

    let decoded_mail_hash = blake3_hash(&buf);
    debug!("decoded mail hash: {}", decoded_mail_hash);

    let decoded_mail_json = serde_json::to_value(&decoded_mail).unwrap();
    let decoded_mail_json_text = serde_json::to_string(&decoded_mail_json).unwrap();

    let compressed_mail = match zstd_compress(&decoded_mail_json_text, 5) {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "compression failed").into_response(),
    };

    let compression_ratio = (compressed_mail.len() as f64) / (decoded_mail_json_text.len() as f64);
    debug!(
        "original: {} bytes, compressed: {} bytes, ratio: {:.2}%",
        decoded_mail_json_text.len(),
        compressed_mail.len(),
        compression_ratio * 100.0
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
        Err(_) => return (StatusCode::SERVICE_UNAVAILABLE, "service error").into_response(),
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

        if original_size > existing_size {
            let update = doc! {
                "$set": {
                    "mail.hash": &decoded_mail_hash,
                    "mail.value": Binary { subtype: BinarySubtype::Generic, bytes: compressed_mail.clone() },
                    "mail.time": mail_time,
                    "mail.id": &mail_id,
                    "metadata.userAgent": user_agent,
                    "metadata.originalSize": original_size,
                },
            };

            let result = mails_collection
                .update_one(doc! { "mail.id": &mail_id }, update)
                .await;

            return match result {
                Ok(_) => (StatusCode::CREATED, "").into_response(),
                Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "service error").into_response(),
            };
        }

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

    let result = mails_collection
        .update_one(doc! { "mail.id": &mail_id }, doc! { "$setOnInsert": doc })
        .upsert(true)
        .await;

    match result {
        Ok(_) => (StatusCode::CREATED, "").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "service error").into_response(),
    }
}
