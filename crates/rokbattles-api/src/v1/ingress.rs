use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use futures_util::{StreamExt, TryStreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const MAX_UPLOAD: usize = 5 * 1024 * 1024; // 5 MB
const BUFFER_LEN: usize = 32;

#[derive(Clone)]
pub struct IngressState {
    pub clamd_addr: String,
}

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

fn has_rok_fileheader(buf: &[u8]) -> bool {
    if buf.len() < BUFFER_LEN {
        return false;
    }
    if buf[0] != 0xFF {
        return false;
    }
    if buf[9] != 0x05 || buf[10] != 0x04 {
        return false;
    }
    let len = {
        let start = 11;
        let end = start + 4;
        let Some(bytes) = buf.get(start..end) else {
            return false;
        };
        u32::from_le_bytes(bytes.try_into().unwrap_or([0; 4]))
    };
    if len != 9 {
        return false;
    }
    let start = 15;
    let end = start + 9;
    let Some(bytes) = buf.get(start..end) else {
        return false;
    };
    bytes == b"mailScene"
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

pub async fn ingress(State(st): State<IngressState>, req: Request<Body>) -> impl IntoResponse {
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

    if !has_rok_fileheader(&peek) {
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

    let first_type = mail_type_detector::detect_type_str(&decoded_mail);
    if !first_type.is_some_and(|t| t.eq_ignore_ascii_case("Battle")) {
        return (StatusCode::UNPROCESSABLE_ENTITY, "not a rok battle mail").into_response();
    }

    // insert into mongodb

    (StatusCode::CREATED, "").into_response()
}
