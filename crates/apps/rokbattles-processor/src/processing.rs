//! Processing loop and mail handling logic.

use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use futures::stream::TryStreamExt;
use mongodb::bson::{Bson, DateTime, Document, oid::ObjectId};
use rokbattles_mail_registry::{MailType, normalize_mail_root, process_mail};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::{debug, error, info};

use crate::{config::Config, error::ProcessorError, storage::Storage};

#[derive(Debug)]
struct RawMail {
    id: ObjectId,
    mail_id: String,
    status: String,
    checksum: String,
    size: i64,
    algorithm: String,
    binary: Vec<u8>,
}

#[derive(Debug)]
struct ObservedVersion {
    checksum: Option<Bson>,
    size: Option<Bson>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessOutcome {
    Processed,
    Stale,
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
                let mail_id = doc
                    .get_document("mail")
                    .ok()
                    .and_then(|mail| mail.get_str("id").ok())
                    .map(str::to_string);
                let raw_id = doc.get_object_id("_id").ok();
                let observed = observed_version(&doc);
                match process_document(&storage, doc).await {
                    Err(error) => {
                        if should_mark_error(&error)
                            && let Some(raw_id) = raw_id
                        {
                            if let Err(mark_error) = storage
                                .mark_error(
                                    &raw_id,
                                    observed.checksum.as_ref(),
                                    observed.size.as_ref(),
                                    DateTime::now(),
                                )
                                .await
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
                    }
                    Ok(ProcessOutcome::Processed) => {
                        processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(ProcessOutcome::Stale) => {}
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

async fn process_document(
    storage: &Storage,
    doc: Document,
) -> Result<ProcessOutcome, ProcessorError> {
    let raw = parse_raw_mail(doc)?;
    let (mail_type, processed_doc) = prepare_processed_document(&raw)?;

    storage
        .upsert_processed(mail_type, &raw.mail_id, &raw.checksum, raw.size, processed_doc)
        .await?;

    let now = DateTime::now();
    if !storage.mark_processed(&raw.id, &raw.checksum, raw.size, now).await? {
        debug!(mail_id = %raw.mail_id, "mail changed while it was being processed");
        return Ok(ProcessOutcome::Stale);
    }
    debug!(mail_id = %raw.mail_id, status = %raw.status, mail_type = %mail_type, "processed mail");

    Ok(ProcessOutcome::Processed)
}

fn prepare_processed_document(raw: &RawMail) -> Result<(MailType, Document), ProcessorError> {
    let decoded = decode_mail_binary(raw)?;
    let root = normalize_mail_root(&decoded).ok_or_else(|| {
        ProcessorError::InvalidMailPayload("mail payload must be an object".to_string())
    })?;
    let mail_type = extract_mail_type(root)?;
    let processed = process_mail(mail_type, root)?;

    let mut processed_doc = mongodb::bson::to_document(&processed)?;
    let metadata = processed_doc
        .get_document_mut("metadata")
        .map_err(|_| ProcessorError::MissingProcessedMetadata)?;
    metadata.insert("source_checksum", raw.checksum.clone());
    metadata.insert("source_size", raw.size);
    Ok((mail_type, processed_doc))
}

fn should_mark_error(error: &ProcessorError) -> bool {
    matches!(
        error,
        ProcessorError::MissingField(_)
            | ProcessorError::MissingProcessedMetadata
            | ProcessorError::UnsupportedCompression(_)
            | ProcessorError::InvalidSize(_)
            | ProcessorError::SizeMismatch { .. }
            | ProcessorError::ChecksumMismatch { .. }
            | ProcessorError::InvalidMailPayload(_)
            | ProcessorError::BinaryDecode(_)
            | ProcessorError::Decompress(_)
            | ProcessorError::UnsupportedMailType(_)
            | ProcessorError::Process(_)
            | ProcessorError::BsonEncode(_)
    )
}

fn parse_raw_mail(mut doc: Document) -> Result<RawMail, ProcessorError> {
    let id = doc.get_object_id("_id").map_err(|_| ProcessorError::MissingField("_id"))?;
    let status = doc.get_str("status").unwrap_or(crate::storage::STATUS_PENDING).to_string();
    let metadata =
        doc.get_document("metadata").map_err(|_| ProcessorError::MissingField("metadata"))?;
    let checksum = metadata
        .get_str("checksum")
        .map_err(|_| ProcessorError::MissingField("metadata.checksum"))?
        .to_string();
    let size =
        metadata.get_i64("size").map_err(|_| ProcessorError::MissingField("metadata.size"))?;
    let algorithm = metadata
        .get_str("algo")
        .map_err(|_| ProcessorError::MissingField("metadata.algo"))?
        .to_string();
    let mut mail = match doc.remove("mail") {
        Some(Bson::Document(mail)) => mail,
        _ => return Err(ProcessorError::MissingField("mail")),
    };
    let mail_id =
        mail.get_str("id").map_err(|_| ProcessorError::MissingField("mail.id"))?.to_string();
    let binary = match mail.remove("binary") {
        Some(Bson::Binary(binary)) => binary.bytes,
        _ => return Err(ProcessorError::MissingField("mail.binary")),
    };

    Ok(RawMail { id, mail_id, status, checksum, size, algorithm, binary })
}

fn decode_mail_binary(raw: &RawMail) -> Result<Value, ProcessorError> {
    if raw.algorithm != "zstd" {
        return Err(ProcessorError::UnsupportedCompression(raw.algorithm.clone()));
    }
    let expected_size =
        usize::try_from(raw.size).map_err(|_| ProcessorError::InvalidSize(raw.size))?;

    let decoder = zstd::stream::read::Decoder::new(raw.binary.as_slice())?;
    let mut bytes = Vec::with_capacity(expected_size);
    decoder.take((expected_size + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() != expected_size {
        return Err(ProcessorError::SizeMismatch { expected: expected_size, actual: bytes.len() });
    }

    let actual_checksum = format!("{:x}", Sha256::digest(&bytes));
    if actual_checksum != raw.checksum {
        return Err(ProcessorError::ChecksumMismatch {
            expected: raw.checksum.clone(),
            actual: actual_checksum,
        });
    }

    Ok(rokbattles_mail_decoder::decode(&bytes)?)
}

fn observed_version(doc: &Document) -> ObservedVersion {
    let metadata = doc.get_document("metadata").ok();
    ObservedVersion {
        checksum: metadata.and_then(|metadata| metadata.get("checksum")).cloned(),
        size: metadata.and_then(|metadata| metadata.get("size")).cloned(),
    }
}

fn extract_mail_type(root: &Value) -> Result<MailType, ProcessorError> {
    if let Some(mail_type) = rokbattles_mail_registry::detect_mail_type(root) {
        return Ok(mail_type);
    }

    let mail_type = rokbattles_mail_registry::raw_mail_type_string(root)
        .ok_or_else(|| ProcessorError::InvalidMailPayload("missing mail type".to_string()))?;
    Err(ProcessorError::UnsupportedMailType(mail_type))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mongodb::bson::{Binary, doc, oid::ObjectId, spec::BinarySubtype};
    use serde_json::json;

    use super::*;
    use crate::storage::STATUS_PENDING;

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
    fn extract_mail_type_parses_scoutreport() {
        let value = json!({ "type": "ScoutReport" });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::ScoutReport);
    }

    #[test]
    fn extract_mail_type_parses_only_gve_member_loot_reports() {
        let gve = json!({
            "type": "EventMemberLootReport",
            "body": { "content": { "EventName": "GVE" } }
        });
        assert_eq!(extract_mail_type(&gve).unwrap(), MailType::EventMemberLootReport);

        let other = json!({
            "type": "EventMemberLootReport",
            "body": { "content": { "EventName": "OtherEvent" } }
        });
        assert!(matches!(extract_mail_type(&other), Err(ProcessorError::UnsupportedMailType(_))));
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
    fn extract_mail_type_parses_system_motte() {
        let value = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 4,
                "subType": 11
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::SystemBarbarianFort);
    }

    #[test]
    fn extract_mail_type_parses_system_kahar_treasure() {
        let value = json!({
            "type": "System",
            "box": "SystemBox",
            "body": {
                "subParam": 11,
                "subType": 29
            }
        });
        let mail_type = extract_mail_type(&value).unwrap();
        assert_eq!(mail_type, MailType::SystemKaharTreasure);
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
    fn extract_mail_type_parses_alliance_type_14_battle_results() {
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
    fn extract_mail_type_rejects_alliance_type_14_battle_results_with_other_param() {
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
    fn extract_mail_type_parses_alliance_type_15_individual_results() {
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
    fn extract_mail_type_rejects_alliance_type_15_individual_results_with_other_param() {
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

    fn raw_mail_from_bytes(bytes: &[u8]) -> RawMail {
        RawMail {
            id: ObjectId::new(),
            mail_id: "mail-1".to_string(),
            status: STATUS_PENDING.to_string(),
            checksum: format!("{:x}", Sha256::digest(bytes)),
            size: i64::try_from(bytes.len()).unwrap(),
            algorithm: "zstd".to_string(),
            binary: zstd::stream::encode_all(Cursor::new(bytes), 3).unwrap(),
        }
    }

    fn native_file(payload: &[u8]) -> Vec<u8> {
        let mut buffer = vec![0xff];
        buffer.extend_from_slice(&0_u64.to_le_bytes());
        buffer.extend_from_slice(payload);
        let checksum =
            buffer.iter().copied().enumerate().fold(0x1505_u64, |hash, (index, byte)| {
                let byte = if (1..9).contains(&index) { 0 } else { byte };
                hash.wrapping_mul(33).wrapping_add(u64::from(byte))
            });
        buffer[1..9].copy_from_slice(&checksum.to_le_bytes());
        buffer
    }

    fn raw_document() -> Document {
        doc! {
            "_id": ObjectId::new(),
            "status": STATUS_PENDING,
            "metadata": {
                "checksum": "abc123",
                "size": 3_i64,
                "algo": "zstd",
            },
            "mail": {
                "id": "mail-1",
                "binary": Binary {
                    subtype: BinarySubtype::Generic,
                    bytes: vec![1, 2, 3],
                }
            }
        }
    }

    #[test]
    fn decode_mail_binary_roundtrips_sample() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/Battle/Persistent.Mail.485440176891031331"
        ));
        let raw = raw_mail_from_bytes(bytes);
        let decoded = decode_mail_binary(&raw).unwrap();
        assert_eq!(rokbattles_mail_registry::detect_mail_type(&decoded), Some(MailType::Battle));
    }

    #[test]
    fn compressed_samples_run_through_every_registered_processor() {
        let samples: &[(&[u8], MailType)] = &[
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/Battle/Persistent.Mail.7948875176322794831"
                )),
                MailType::Battle,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/DuelBattle2/Persistent.Mail.4198599176618253831"
                )),
                MailType::DuelBattle2,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/BarCanyonKillBoss/Persistent.Mail.83062859177409782917"
                )),
                MailType::BarCanyonKillBoss,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/EventMemberLootReport/Persistent.Mail.28722408178369207531"
                )),
                MailType::EventMemberLootReport,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/Rss/Persistent.Mail.118801516499340535"
                )),
                MailType::Rss,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/ScoutReport/Persistent.Mail.137024509177843958431"
                )),
                MailType::ScoutReport,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/System/Persistent.Mail.6603502177237171628"
                )),
                MailType::SystemBarbarianFort,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/System/Persistent.Mail.22165348178347040031"
                )),
                MailType::SystemKaharTreasure,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/Alliance/Persistent.Mail.102185423177177256731"
                )),
                MailType::AllianceAOOBattleResults,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/Alliance/Persistent.Mail.102185425177177256731"
                )),
                MailType::AllianceAOOBattleInfo,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/Alliance/Persistent.Mail.102185429177177256731"
                )),
                MailType::AllianceAOOIndividualResults,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/Alliance/Persistent.Mail.108518435177768053226"
                )),
                MailType::AllianceAOORegistration,
            ),
        ];

        for (bytes, expected_type) in samples {
            let raw = raw_mail_from_bytes(bytes);
            let (mail_type, processed) =
                prepare_processed_document(&raw).expect("process compressed sample");
            assert_eq!(mail_type, *expected_type);
            assert_eq!(
                processed.get_document("metadata").unwrap().get_i64("source_size").unwrap(),
                raw.size
            );
            let metadata = processed.get_document("metadata").unwrap();
            assert_eq!(metadata.get_str("source_checksum").unwrap(), raw.checksum);
        }
    }

    #[test]
    fn all_processable_binary_samples_match_processed_fixtures() {
        let samples_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../samples");
        let mut samples = Vec::new();
        collect_binary_mail_samples(&samples_root, &mut samples);
        samples.sort();
        let mut processed_count = 0;

        for input in samples {
            let bytes = std::fs::read(&input)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", input.display()));
            let decoded = rokbattles_mail_decoder::decode(&bytes)
                .unwrap_or_else(|error| panic!("failed to decode {}: {error}", input.display()));
            let Some(root) = normalize_mail_root(&decoded) else {
                panic!("non-object root for {}", input.display());
            };
            let Some(mail_type) = rokbattles_mail_registry::detect_mail_type(root) else {
                continue;
            };
            let processed = process_mail(mail_type, root)
                .unwrap_or_else(|error| panic!("failed to process {}: {error}", input.display()));
            let actual = serde_json::to_value(processed).expect("serialize processed mail");
            let expected_path =
                std::path::PathBuf::from(format!("{}-processed.json", input.display()));
            let expected: Value =
                serde_json::from_slice(&std::fs::read(&expected_path).unwrap_or_else(|error| {
                    panic!("failed to read {}: {error}", expected_path.display())
                }))
                .unwrap_or_else(|error| {
                    panic!("failed to parse {}: {error}", expected_path.display())
                });

            assert!(
                json_equivalent(&actual, &expected),
                "processed output differs for {}",
                input.display()
            );
            processed_count += 1;
        }

        assert_eq!(processed_count, 135);
    }

    fn collect_binary_mail_samples(root: &std::path::Path, samples: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(root)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        {
            let path = entry
                .unwrap_or_else(|error| {
                    panic!("failed to read entry in {}: {error}", root.display())
                })
                .path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("game") {
                    collect_binary_mail_samples(&path, samples);
                }
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with("Persistent.Mail.") && !name.ends_with(".json") {
                samples.push(path);
            }
        }
    }

    fn json_equivalent(actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Array(actual), Value::Array(expected)) => {
                actual.len() == expected.len()
                    && actual.iter().zip(expected).all(|(a, b)| json_equivalent(a, b))
            }
            (Value::Object(actual), Value::Object(expected)) => {
                actual.len() == expected.len()
                    && actual.iter().all(|(key, value)| {
                        expected.get(key).is_some_and(|other| json_equivalent(value, other))
                    })
            }
            (Value::Number(actual), Value::Number(expected)) => {
                match (actual.as_i64(), expected.as_i64()) {
                    (Some(actual), Some(expected)) => actual == expected,
                    _ => match (actual.as_u64(), expected.as_u64()) {
                        (Some(actual), Some(expected)) => actual == expected,
                        _ => match (actual.as_f64(), expected.as_f64()) {
                            (Some(actual), Some(expected)) => {
                                ordered_float_bits(actual).abs_diff(ordered_float_bits(expected))
                                    <= 1
                            }
                            _ => false,
                        },
                    },
                }
            }
            _ => actual == expected,
        }
    }

    fn ordered_float_bits(value: f64) -> u64 {
        let bits = value.to_bits();
        if bits & (1_u64 << 63) == 0 { bits | (1_u64 << 63) } else { !bits }
    }

    #[test]
    fn decode_mail_binary_rejects_unsupported_algorithm() {
        let mut raw = raw_mail_from_bytes(b"anything");
        raw.algorithm = "gzip".to_string();
        let error = decode_mail_binary(&raw).unwrap_err();
        assert!(matches!(error, ProcessorError::UnsupportedCompression(value) if value == "gzip"));
    }

    #[test]
    fn decode_mail_binary_rejects_negative_size() {
        let mut raw = raw_mail_from_bytes(b"anything");
        raw.size = -1;
        let error = decode_mail_binary(&raw).unwrap_err();
        assert!(matches!(error, ProcessorError::InvalidSize(-1)));
    }

    #[test]
    fn decode_mail_binary_rejects_size_mismatch() {
        let mut raw = raw_mail_from_bytes(b"anything");
        raw.size += 1;
        let error = decode_mail_binary(&raw).unwrap_err();
        assert!(matches!(error, ProcessorError::SizeMismatch { expected: 9, actual: 8 }));
    }

    #[test]
    fn decode_mail_binary_rejects_checksum_mismatch() {
        let mut raw = raw_mail_from_bytes(b"anything");
        raw.checksum = "wrong".to_string();
        let error = decode_mail_binary(&raw).unwrap_err();
        assert!(
            matches!(error, ProcessorError::ChecksumMismatch { expected, .. } if expected == "wrong")
        );
    }

    #[test]
    fn decode_mail_binary_rejects_corrupt_zstd() {
        let mut raw = raw_mail_from_bytes(b"anything");
        raw.binary = vec![1, 2, 3];
        let error = decode_mail_binary(&raw).unwrap_err();
        assert!(matches!(error, ProcessorError::Decompress(_)));
    }

    #[test]
    fn decode_mail_binary_rejects_invalid_binary_mail() {
        let raw = raw_mail_from_bytes(&[]);
        let error = decode_mail_binary(&raw).unwrap_err();
        assert!(matches!(error, ProcessorError::BinaryDecode(_)));
    }

    #[test]
    fn prepare_processed_document_rejects_non_object_root() {
        let bytes = native_file(&[0x01, 1]);
        let raw = raw_mail_from_bytes(&bytes);
        let error = prepare_processed_document(&raw).unwrap_err();
        assert!(matches!(error, ProcessorError::InvalidMailPayload(_)));
    }

    #[test]
    fn parse_raw_mail_reads_fields() {
        let raw = parse_raw_mail(raw_document()).unwrap();
        assert_eq!(raw.mail_id, "mail-1");
        assert_eq!(raw.status, "pending");
        assert_eq!(raw.checksum, "abc123");
        assert_eq!(raw.size, 3);
        assert_eq!(raw.algorithm, "zstd");
        assert_eq!(raw.binary, vec![1, 2, 3]);
    }

    #[test]
    fn parse_raw_mail_requires_mail_id() {
        let mut doc = raw_document();
        doc.get_document_mut("mail").unwrap().remove("id");
        let err = parse_raw_mail(doc).unwrap_err();
        assert!(matches!(err, ProcessorError::MissingField("mail.id")));
    }

    #[test]
    fn parse_raw_mail_requires_v2_metadata_fields_and_binary() {
        let cases = [
            ("metadata.checksum", "metadata", "checksum"),
            ("metadata.size", "metadata", "size"),
            ("metadata.algo", "metadata", "algo"),
            ("mail.binary", "mail", "binary"),
        ];

        for (expected, section, field) in cases {
            let mut doc = raw_document();
            doc.get_document_mut(section).unwrap().remove(field);
            let error = parse_raw_mail(doc).unwrap_err();
            assert!(matches!(error, ProcessorError::MissingField(field) if field == expected));
        }
    }

    #[test]
    fn observed_version_preserves_exact_bson_values() {
        let doc = doc! { "metadata": { "checksum": 7_i32, "size": "bad" } };
        let observed = observed_version(&doc);
        assert_eq!(observed.checksum, Some(Bson::Int32(7)));
        assert_eq!(observed.size, Some(Bson::String("bad".to_string())));
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
