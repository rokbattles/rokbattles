use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};

use crate::error::RokGtError;

pub(crate) fn signed_passport_url(
    base_url: &str,
    path: &str,
    access_key: &str,
    secret_key: &str,
) -> Result<String, RokGtError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RokGtError::InvalidSystemTime)?
        .as_millis()
        .to_string();
    let signature = md5_hex(format!("access_key={access_key}timestamp={timestamp}{secret_key}"));
    Ok(format!(
        "{base_url}{path}?timestamp={timestamp}&signature={signature}&access_key={access_key}"
    ))
}

pub(crate) fn client_info(visitor_id: &str) -> String {
    format!("os-type=pc;language=en;visitor-id={visitor_id}")
}

pub(crate) fn normalize_bearer_token(token: impl Into<String>) -> String {
    let token = token.into();
    token.trim().strip_prefix("Bearer ").unwrap_or(token.trim()).to_string()
}

pub(crate) fn md5_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Md5::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

pub(crate) fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}
