//! Processing loop and mail handling logic.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures::stream::TryStreamExt;
use mongodb::bson::{Bson, DateTime, Document, oid::ObjectId};
use serde_json::Value;
use tracing::{debug, error, info};

use crate::{config::Config, error::ProcessorError, mail::MailType, storage::Storage};

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
                let raw_id = doc.get_object_id("_id").ok();
                if let Err(error) = process_document(&storage, doc).await {
                    if should_mark_error(&error)
                        && let Some(raw_id) = raw_id
                    {
                        if let Err(mark_error) = storage.mark_error(&raw_id, DateTime::now()).await
                        {
                            error!(
                                error = %error,
                                mark_error = %mark_error,
                                mail_id = mail_id.as_deref().unwrap_or("unknown"),
                                "processing mail failed and status update failed"
                            );
                        } else if let Some(mail_id) = mail_id {
                            error!(error = %error, mail_id = %mail_id, "processing mail failed");
                        } else {
                            error!(error = %error, "processing mail failed");
                        }
                    } else if let Some(mail_id) = mail_id {
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
        MailType::Battle => mail_processor_battle::process(root)?,
        MailType::DuelBattle2 => mail_processor_duelbattle2::process(root)?,
        MailType::BarCanyonKillBoss => mail_processor_barcanyonkillboss::process(root)?,
        MailType::Rss => mail_processor_rss::process(root)?,
        MailType::SystemBarbarianFort => mail_processor_system_barbarianfort::process(root)?,
        MailType::AllianceAOOBattleResults => {
            mail_processor_alliance_aoo_battle_results::process(root)?
        }
        MailType::AllianceAOOBattleInfo => mail_processor_alliance_aoo_battle_info::process(root)?,
        MailType::AllianceAOOIndividualResults => {
            mail_processor_alliance_aoo_individual_results::process(root)?
        }
    };

    let processed_doc = mongodb::bson::to_document(&processed)?;
    storage.upsert_processed(mail_type, &raw.mail_id, processed_doc).await?;

    let now = DateTime::now();
    storage.mark_processed(&raw.id, now).await?;
    debug!(mail_id = %raw.mail_id, status = %raw.status, mail_type = %mail_type, "processed mail");

    Ok(())
}

fn should_mark_error(error: &ProcessorError) -> bool {
    matches!(
        error,
        ProcessorError::MissingField(_)
            | ProcessorError::InvalidMailPayload(_)
            | ProcessorError::Decode(_)
            | ProcessorError::Decompress(_)
            | ProcessorError::UnsupportedMailType(_)
            | ProcessorError::Process(_)
            | ProcessorError::BsonEncode(_)
    )
}

fn parse_raw_mail(doc: Document) -> Result<RawMail, ProcessorError> {
    let id = doc.get_object_id("_id").map_err(|_| ProcessorError::MissingField("_id"))?;
    let mail_id =
        doc.get_str("mail_id").map_err(|_| ProcessorError::MissingField("mail_id"))?.to_string();
    let status = doc.get_str("status").unwrap_or(crate::storage::STATUS_PENDING).to_string();
    let mail_value = match doc.get("mail_value") {
        Some(Bson::Binary(binary)) => binary.bytes.clone(),
        _ => return Err(ProcessorError::MissingField("mail_value")),
    };

    Ok(RawMail { id, mail_id, status, mail_value })
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
    if is_system_barbarian_fort_mail(root) {
        return Ok(MailType::SystemBarbarianFort);
    }
    if let Some(mail_type) = detect_alliance_aoo_mail_type(root) {
        return Ok(mail_type);
    }

    let mail_type = root
        .get("type")
        .and_then(value_to_string)
        .ok_or_else(|| ProcessorError::InvalidMailPayload("missing mail type".to_string()))?;
    MailType::from_str(&mail_type).ok_or_else(|| ProcessorError::UnsupportedMailType(mail_type))
}

fn is_system_barbarian_fort_mail(root: &Value) -> bool {
    let Some(root) = root.as_object() else {
        return false;
    };
    if !matches!(root.get("type").and_then(Value::as_str), Some("System")) {
        return false;
    }
    if !matches!(root.get("box").and_then(Value::as_str), Some("Report")) {
        return false;
    }

    let Some(body) = root.get("body").and_then(Value::as_object) else {
        return false;
    };
    let sub_param = body.get("subParam").and_then(value_as_u64);
    let sub_type = body.get("subType").and_then(value_as_u64);
    matches!(sub_type, Some(11)) && matches!(sub_param, Some(1 | 3))
}

fn detect_alliance_aoo_mail_type(root: &Value) -> Option<MailType> {
    let root = root.as_object()?;
    if !matches!(root.get("type").and_then(Value::as_str), Some("Alliance")) {
        return None;
    }
    if !matches!(root.get("box").and_then(Value::as_str), Some("AllianceBox")) {
        return None;
    }

    let body = root.get("body").and_then(Value::as_object)?;
    let body_type = body.get("type").and_then(value_as_u64)?;
    let body_param = body.get("param").and_then(value_as_u64);

    match body_type {
        // custom Ark match
        14 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOOBattleResults),
        15 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOOIndividualResults),
        // normal Ark match
        60 => Some(MailType::AllianceAOOBattleResults),
        61 => Some(MailType::AllianceAOOBattleInfo),
        62 => Some(MailType::AllianceAOOIndividualResults),
        _ => None,
    }
}

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
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
    use std::io::Cursor;

    use mongodb::bson::{Binary, doc, oid::ObjectId, spec::BinarySubtype};
    use serde_json::json;

    use super::*;

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
    fn extract_mail_type_parses_rss() {
        let value = json!({ "type": "Rss" });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::Rss);
    }

    #[test]
    fn extract_mail_type_parses_system_barbarian_fort() {
        let value = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 1,
                "subType": 11
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::SystemBarbarianFort);
    }

    #[test]
    fn extract_mail_type_parses_system_barbarian_fort_with_sub_param_three() {
        let value = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 3,
                "subType": 11
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::SystemBarbarianFort);
    }

    #[test]
    fn extract_mail_type_rejects_system_mail_with_unsupported_sub_param() {
        let value = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 2,
                "subType": 11
            }
        });
        let err = extract_mail_type(&value).unwrap_err();
        assert!(matches!(
            err,
            ProcessorError::UnsupportedMailType(mail_type) if mail_type == "System"
        ));
    }

    #[test]
    fn extract_mail_type_parses_alliance_aoo_battle_results() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 60
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::AllianceAOOBattleResults);
    }

    #[test]
    fn extract_mail_type_parses_alliance_custom_battle_results() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 14,
                "param": 1
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::AllianceAOOBattleResults);
    }

    #[test]
    fn extract_mail_type_rejects_alliance_custom_battle_results_with_other_param() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 14,
                "param": 2
            }
        });
        let err = extract_mail_type(&value).unwrap_err();
        assert!(
            matches!(err, ProcessorError::UnsupportedMailType(mail_type) if mail_type == "Alliance")
        );
    }

    #[test]
    fn extract_mail_type_parses_alliance_aoo_battle_info() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 61
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::AllianceAOOBattleInfo);
    }

    #[test]
    fn extract_mail_type_parses_alliance_aoo_individual_results() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 62
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::AllianceAOOIndividualResults);
    }

    #[test]
    fn extract_mail_type_parses_alliance_custom_individual_results() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 15,
                "param": 1
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::AllianceAOOIndividualResults);
    }

    #[test]
    fn extract_mail_type_rejects_alliance_custom_individual_results_with_other_param() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 15,
                "param": 3
            }
        });
        let err = extract_mail_type(&value).unwrap_err();
        assert!(
            matches!(err, ProcessorError::UnsupportedMailType(mail_type) if mail_type == "Alliance")
        );
    }

    #[test]
    fn extract_mail_type_keeps_regular_alliance_unsupported() {
        let value = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": {
                "type": 99
            }
        });
        let err = extract_mail_type(&value).unwrap_err();
        assert!(matches!(
            err,
            ProcessorError::UnsupportedMailType(mail_type)
                if mail_type == "Alliance"
        ));
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

    #[test]
    fn should_mark_error_for_unprocessable_mail() {
        let err = ProcessorError::UnsupportedMailType("Unknown".to_string());
        assert!(should_mark_error(&err));
    }

    #[test]
    fn should_not_mark_error_for_transient_storage_failures() {
        let mongo_err = mongodb::error::Error::custom("temporary mongo write failure");
        let err = ProcessorError::Mongo(mongo_err);
        assert!(!should_mark_error(&err));
    }
}
