use axum::http::StatusCode;

pub async fn ingress() -> StatusCode {
    // TODO implement me
    // check user-agent (prefix + ver)
    // check for signals (ver, tauri version, etc)
    // check content-type (raw)
    // check content-encoding (identity)
    // check content-length (required; len <= max upload size)
    // validate buf len against content-length (must match)
    // peek first 32B to validate rok header
    // stream to clamd
    // decode
    // save to db
    StatusCode::BAD_REQUEST
}
