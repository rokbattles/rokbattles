//! Shared insert-or-skip workflow for immutable mail uploads.
//!
//! HTTP validation and ClamAV scanning belong to the routes. Existing IDs are skipped
//! before document construction; the storage layer's unique index handles races.

pub(crate) mod metadata;

use mongodb::bson::DateTime;
use rokbattles_mail_registry::is_processable_mail_type;
use serde_json::Value;

use self::metadata::{extract_mail_id, extract_mail_type, extract_metadata};
use crate::{
    error::ApiError,
    storage::{
        Storage,
        document::{self, DocumentInput},
    },
};

const STATUS_PENDING: &str = "pending";
const STATUS_UNPROCESSABLE: &str = "unprocessable";

/// Whether ingress inserted the mail or found its ID already stored.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StoreOutcome {
    Stored,
    Skipped,
}

impl StoreOutcome {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::Skipped => "skipped",
        }
    }
}

/// Checks the reconstructed mail ID, then stores or skips the mail.
///
/// Expects output from [`rokbattles_mail_reconstructor::MailReconstructor::reconstruct`],
/// which has already checked the mail category. Decoding and the ID check run
/// before duplicate lookup.
pub(crate) async fn store_reconstructed(
    storage: &Storage,
    bytes: &[u8],
    mail_id: &str,
    user_agent: &str,
) -> Result<StoreOutcome, ApiError> {
    let decoded = rokbattles_mail_codec::decode(bytes)
        .map_err(|error| ApiError::decode_failed(error.to_string()))?;

    let decoded_id =
        extract_mail_id(&decoded).ok_or_else(|| ApiError::bad_request("missing mail id"))?;
    if decoded_id != mail_id {
        return Err(ApiError::bad_request("reconstructed mail id mismatch"));
    }

    let mail_type = extract_mail_type(&decoded)?;

    store(storage, bytes, &decoded, mail_id, &mail_type, user_agent).await
}

/// Stores a new mail, leaving any document with the same ID unchanged.
///
/// Callers must validate the upload and pass its matching bytes, decoded value,
/// ID, and type. This function does not recheck those relationships.
///
/// Existing IDs skip metadata extraction and compression. A concurrent insert
/// of the same ID also returns [`StoreOutcome::Skipped`].
pub(crate) async fn store(
    storage: &Storage,
    buffer: &[u8],
    decoded: &Value,
    mail_id: &str,
    mail_type: &str,
    user_agent: &str,
) -> Result<StoreOutcome, ApiError> {
    if storage
        .compressed_raw_exists(mail_id)
        .await
        .map_err(|error| ApiError::database(error.to_string()))?
    {
        return Ok(StoreOutcome::Skipped);
    }

    let mail = extract_metadata(decoded)?;
    let doc = document::build(DocumentInput {
        original_bytes: buffer,
        user_agent,
        mail: &mail,
        status: insert_status_for_mail_type(mail_type),
        now: DateTime::now(),
    })?;

    let inserted = storage
        .insert_compressed_raw(mail_id, doc)
        .await
        .map_err(|error| ApiError::database(error.to_string()))?;

    Ok(if inserted { StoreOutcome::Stored } else { StoreOutcome::Skipped })
}

fn insert_status_for_mail_type(mail_type: &str) -> &'static str {
    if is_processable_mail_type(mail_type) { STATUS_PENDING } else { STATUS_UNPROCESSABLE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping_marks_supported_processor_types_processable() {
        assert_eq!(insert_status_for_mail_type("Battle"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("Rss"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("SystemBarbarianFort"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("SystemKaharTreasure"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("AllianceAOOBattleResults"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("AllianceAOOBattleInfo"), STATUS_PENDING);
        assert_eq!(insert_status_for_mail_type("AllianceAOOIndividualResults"), STATUS_PENDING);
    }
}
