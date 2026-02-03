//! Processing loop and mail handling logic.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::stream::TryStreamExt;
use mongodb::bson::{Bson, DateTime, Document, oid::ObjectId};
use serde_json::Value;
use tracing::{debug, error, info};

use crate::config::Config;
use crate::error::ProcessorError;
use crate::mail::MailType;
use crate::storage::Storage;

#[derive(Debug)]
struct RawMail {
    id: ObjectId,
    mail_id: String,
    status: String,
    mail_value: Vec<u8>,
}

/// Run the processor loop forever.
pub async fn process_loop(storage: Storage, config: Config) -> Result<(), ProcessorError> {
    loop {
        match process_batch(&storage, &config).await {
            Ok(0) => tokio::time::sleep(config.idle_sleep).await,
            Ok(_) => {}
            Err(error) => {
                error!(error = %error, "processing batch failed");
                tokio::time::sleep(config.idle_sleep).await;
            }
        }
    }
}

async fn process_batch(storage: &Storage, config: &Config) -> Result<usize, ProcessorError> {
    let cursor = storage.find_pending(config.batch_size).await?;
    let processed = Arc::new(AtomicUsize::new(0));

    cursor
        .try_for_each_concurrent(config.concurrency, |doc| {
            let storage = storage.clone();
            let processed = Arc::clone(&processed);
            async move {
                let mail_id = doc.get_str("mail_id").ok().map(str::to_string);
                if let Err(error) = process_document(&storage, doc).await {
                    if let Some(mail_id) = mail_id {
                        error!(error = %error, mail_id = %mail_id, "processing mail failed");
                    } else {
                        error!(error = %error, "processing mail failed");
                    }
                } else {
                    processed.fetch_add(1, Ordering::Relaxed);
                }
                Ok(())
            }
        })
        .await?;

    let processed_count = processed.load(Ordering::Relaxed);
    if processed_count > 0 {
        info!(processed_count, "processed mails");
    } else {
        debug!("no pending mails");
    }

    Ok(processed_count)
}

async fn process_document(storage: &Storage, doc: Document) -> Result<(), ProcessorError> {
    let raw = parse_raw_mail(doc)?;
    let decoded = decode_mail_value(&raw.mail_value)?;
    let root = normalize_root(&decoded).ok_or_else(|| {
        ProcessorError::InvalidMailPayload("mail payload must be an object".to_string())
    })?;
    let mail_type = extract_mail_type(root)?;
    let processed = match mail_type {
        MailType::Battle => mail_processor_battle::process_parallel(root)?,
        MailType::DuelBattle2 => mail_processor_duelbattle2::process_parallel(root)?,
        MailType::BarCanyonKillBoss => mail_processor_barcanyonkillboss::process_parallel(root)?,
    };

    let processed_doc = mongodb::bson::to_document(&processed)?;
    storage
        .upsert_processed(mail_type, &raw.mail_id, processed_doc)
        .await?;

    let now = DateTime::now();
    storage.mark_processed(&raw.id, now).await?;
    debug!(mail_id = %raw.mail_id, status = %raw.status, mail_type = %mail_type, "processed mail");

    Ok(())
}

fn parse_raw_mail(doc: Document) -> Result<RawMail, ProcessorError> {
    let id = doc
        .get_object_id("_id")
        .map_err(|_| ProcessorError::MissingField("_id"))?;
    let mail_id = doc
        .get_str("mail_id")
        .map_err(|_| ProcessorError::MissingField("mail_id"))?
        .to_string();
    let status = doc
        .get_str("status")
        .unwrap_or(crate::storage::STATUS_PENDING)
        .to_string();
    let mail_value = match doc.get("mail_value") {
        Some(Bson::Binary(binary)) => binary.bytes.clone(),
        _ => return Err(ProcessorError::MissingField("mail_value")),
    };

    Ok(RawMail {
        id,
        mail_id,
        status,
        mail_value,
    })
}

fn decode_mail_value(bytes: &[u8]) -> Result<Value, ProcessorError> {
    let decoded = zstd::decode_all(bytes)?;
    Ok(serde_json::from_slice(&decoded)?)
}

fn normalize_root(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(_) => Some(value),
        Value::Array(items) => match items.as_slice() {
            [item] if item.is_object() => Some(item),
            _ => None,
        },
        _ => None,
    }
}

fn extract_mail_type(root: &Value) -> Result<MailType, ProcessorError> {
    let mail_type = root
        .get("type")
        .and_then(value_to_string)
        .ok_or_else(|| ProcessorError::InvalidMailPayload("missing mail type".to_string()))?;
    MailType::from_str(&mail_type).ok_or_else(|| ProcessorError::UnsupportedMailType(mail_type))
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::{Binary, doc, oid::ObjectId, spec::BinarySubtype};
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn normalize_root_accepts_object() {
        let value = json!({ "type": "Battle" });
        assert!(normalize_root(&value).is_some());
    }

    #[test]
    fn normalize_root_accepts_singleton_array() {
        let value = json!([{ "type": "Battle" }]);
        assert!(normalize_root(&value).is_some());
    }

    #[test]
    fn normalize_root_rejects_other_shapes() {
        let value = json!([1, 2, 3]);
        assert!(normalize_root(&value).is_none());
    }

    #[test]
    fn extract_mail_type_parses_known_types() {
        let value = json!({ "type": "DuelBattle2" });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::DuelBattle2);
    }

    #[test]
    fn decode_mail_value_roundtrip() {
        let payload = json!({ "type": "Battle", "id": "mail-1" });
        let json_bytes = serde_json::to_vec(&payload).unwrap();
        let compressed = zstd::stream::encode_all(Cursor::new(json_bytes), 3).unwrap();
        let decoded = decode_mail_value(&compressed).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn parse_raw_mail_reads_fields() {
        let id = ObjectId::new();
        let doc = doc! {
            "_id": id,
            "mail_id": "mail-1",
            "status": "pending",
            "mail_value": Binary {
                subtype: BinarySubtype::Generic,
                bytes: vec![1, 2, 3],
            }
        };
        let raw = parse_raw_mail(doc).unwrap();
        assert_eq!(raw.mail_id, "mail-1");
        assert_eq!(raw.status, "pending");
        assert_eq!(raw.mail_value, vec![1, 2, 3]);
    }

    #[test]
    fn parse_raw_mail_requires_mail_id() {
        let id = ObjectId::new();
        let doc = doc! {
            "_id": id,
            "mail_value": Binary {
                subtype: BinarySubtype::Generic,
                bytes: vec![1, 2, 3],
            }
        };
        let err = parse_raw_mail(doc).unwrap_err();
        assert!(matches!(err, ProcessorError::MissingField("mail_id")));
    }
}
