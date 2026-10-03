//! Authenticated relay batches, reconstructed and stored independently per entry.
//!
//! Relay uploads are reconstructed without a ClamAV scan.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use bytes::Bytes;
use rokbattles_mail_reconstructor::ReconstructionContext;
use serde::Serialize;

use super::headers::{extract_user_agent, is_allowed_content_type};
use crate::{error::ApiError, mail, state::AppState};

const MAX_RELAY_BATCH_ENTRIES: usize = 512;

/// Per-entry result returned by the relay batch endpoint.
///
/// IDs and types are included only after successful reconstruction. An error
/// message is included only for rejected entries; absent fields are omitted.
#[derive(Debug, Serialize)]
struct RelayMailResult {
    /// Zero-based position among the batch's `mail` fields.
    index: usize,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    mail_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mail_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Response returned after independently processing every batch entry.
#[derive(Debug, Serialize)]
struct RelayBatchResponse {
    results: Vec<RelayMailResult>,
}

/// Reconstructs a relay batch and returns a stored, skipped, or rejected result per entry.
///
/// Entries are processed in order without a transaction: rejecting one does not
/// roll back earlier inserts. Once the batch is parsed, the response is `200`
/// even if every entry is rejected, so callers must inspect the results.
pub(super) async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    authorize_relay(&headers, &state.config.relay_token)?;
    let user_agent = extract_user_agent(&headers)?;
    let batch = read_batch(&mut multipart).await?;
    let mut results = Vec::with_capacity(batch.entries.len());

    for (index, entry) in batch.entries.into_iter().enumerate() {
        let reconstructed = match state.mail_reconstructor.reconstruct(&entry, batch.context) {
            Ok(mail) => mail,
            Err(error) => {
                results.push(RelayMailResult {
                    index,
                    status: "rejected",
                    mail_id: None,
                    mail_type: None,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };

        let (status, error) = match mail::store_reconstructed(
            &state.storage,
            &reconstructed.bytes,
            &reconstructed.id,
            user_agent,
        )
        .await
        {
            Ok(outcome) => (outcome.label(), None),
            Err(error) => ("rejected", Some(error.to_string())),
        };

        results.push(RelayMailResult {
            index,
            status,
            mail_id: Some(reconstructed.id),
            mail_type: Some(reconstructed.mail_type),
            error,
        });
    }

    Ok((StatusCode::OK, Json(RelayBatchResponse { results })))
}

struct RelayBatch {
    context: ReconstructionContext,
    entries: Vec<Bytes>,
}

/// Parses the whole batch before processing entries, so envelope errors fail the request.
///
/// Context fields apply to every entry regardless of their position in the body.
/// If a context field occurs more than once, its last value wins.
async fn read_batch(multipart: &mut Multipart) -> Result<RelayBatch, ApiError> {
    let mut server_id = None;
    let mut player_id = None;
    let mut entries = Vec::new();

    while let Some(field) =
        multipart.next_field().await.map_err(|error| ApiError::bad_request(error.to_string()))?
    {
        match field.name() {
            Some("server_id") => {
                server_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?
                        .parse::<i32>()
                        .map_err(|_error| ApiError::bad_request("server_id must be an i32"))?,
                );
            }
            Some("player_id") => {
                player_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?
                        .parse::<i64>()
                        .map_err(|_error| ApiError::bad_request("player_id must be an i64"))?,
                );
            }
            Some("mail") => {
                if entries.len() >= MAX_RELAY_BATCH_ENTRIES {
                    return Err(ApiError::bad_request(format!(
                        "relay batch exceeds {MAX_RELAY_BATCH_ENTRIES} entries"
                    )));
                }

                let content_type = field.content_type().unwrap_or("application/octet-stream");
                if !is_allowed_content_type(content_type) {
                    return Err(ApiError::bad_request(format!(
                        "unsupported relay mail content type: {content_type}"
                    )));
                }

                entries.push(
                    field
                        .bytes()
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?,
                );
            }
            Some(name) => {
                return Err(ApiError::bad_request(format!(
                    "unsupported relay batch field: {name}"
                )));
            }
            None => return Err(ApiError::bad_request("relay batch field is missing a name")),
        }
    }

    if entries.is_empty() {
        return Err(ApiError::bad_request("relay batch contains no mail entries"));
    }

    Ok(RelayBatch { context: ReconstructionContext { server_id, player_id }, entries })
}

fn authorize_relay(headers: &HeaderMap, expected: &str) -> Result<(), ApiError> {
    let actual = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    if constant_time_eq(actual.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    // Include lengths so zero-padding a shorter token cannot make it compare equal.
    let mut difference = left.len() ^ right.len();

    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        difference |= usize::from(left_byte ^ right_byte);
    }

    difference == 0
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;
    use crate::routes::tests::multipart;

    #[tokio::test]
    async fn reads_relay_context_after_mail_entries() {
        let mut multipart = multipart(concat!(
            "--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"mail\"\r\n\r\n",
            "first-mail\r\n--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"mail\"\r\n\r\n",
            "second-mail\r\n--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"server_id\"\r\n\r\n",
            "1234\r\n--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"player_id\"\r\n\r\n",
            "5678\r\n--mail-boundary--\r\n",
        ))
        .await;

        let batch = read_batch(&mut multipart).await.expect("read relay batch");

        assert_eq!(batch.context.server_id, Some(1234));
        assert_eq!(batch.context.player_id, Some(5678));
        assert_eq!(
            batch.entries,
            [Bytes::from_static(b"first-mail"), Bytes::from_static(b"second-mail")]
        );
    }

    #[tokio::test]
    async fn rejects_unknown_relay_field() {
        let mut multipart = multipart(concat!(
            "--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"unexpected\"\r\n\r\n",
            "value\r\n--mail-boundary--\r\n",
        ))
        .await;

        assert!(matches!(
            read_batch(&mut multipart).await,
            Err(ApiError::BadRequest(message))
                if message == "unsupported relay batch field: unexpected"
        ));
    }

    #[tokio::test]
    async fn enforces_relay_entry_limit() {
        let entry = concat!(
            "--mail-boundary\r\n",
            "Content-Disposition: form-data; name=\"mail\"\r\n\r\n",
            "mail\r\n",
        );
        let at_limit = format!("{}--mail-boundary--\r\n", entry.repeat(512));
        let over_limit = format!("{}--mail-boundary--\r\n", entry.repeat(513));

        let batch = read_batch(&mut multipart(&at_limit).await).await.expect("512 entries");
        assert_eq!(batch.entries.len(), 512);
        assert!(matches!(
            read_batch(&mut multipart(&over_limit).await).await,
            Err(ApiError::BadRequest(message)) if message == "relay batch exceeds 512 entries"
        ));
    }

    #[test]
    fn relay_authorization_requires_matching_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer secret"));

        authorize_relay(&headers, "secret").expect("operation should succeed");
        assert!(matches!(authorize_relay(&headers, "different"), Err(ApiError::Unauthorized)));
    }
}
