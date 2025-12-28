use anyhow::{Context, Result, bail};
use blake3::Hasher;
use futures::stream::TryStreamExt;
use mail_helper::EmailType;
use mongodb::{
    Collection, Database, bson,
    bson::Bson,
    bson::DateTime,
    bson::{Document, doc},
    error::ErrorKind,
    options::FindOneAndUpdateOptions,
    options::FindOptions,
    options::InsertManyOptions,
    options::ReturnDocument,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn blake3_hash(data: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hash.to_hex().to_string()
}

async fn process(
    db: &Database,
    batch_size: i64,
    concurrency: usize,
    max_failures: i64,
) -> Result<usize> {
    let mails = db.collection::<Document>("mails");
    let battle_reports = db.collection::<Document>("battleReports");
    let duelbattle2_reports = db.collection::<Document>("duelbattle2Reports");

    let filter = doc! {
        "status": { "$in": ["pending", "reprocess"] },
    };

    // Pull a batch of raw reports at a time.
    let opts = FindOptions::builder()
        .limit(batch_size)
        .sort(doc! { "mail.time": 1 })
        .projection(doc! {
            "_id": 1,
            "mail.hash": 1,
            "mail.codec": 1,
            "mail.value": 1,
            "status": 1,
            "metadata.previousHashes": 1,
        })
        .build();

    let cursor = mails.find(filter).with_options(opts).await?;
    let count = Arc::new(AtomicUsize::new(0));
    let count_ref = Arc::clone(&count);
    let mails_ref = mails.clone();
    let battle_reports_ref = battle_reports.clone();
    let duelbattle2_reports_ref = duelbattle2_reports.clone();

    cursor
        .try_for_each_concurrent(concurrency, move |doc| {
            let mails_ref = mails_ref.clone();
            let battle_reports_ref = battle_reports_ref.clone();
            let duelbattle2_reports_ref = duelbattle2_reports_ref.clone();
            let count_ref = Arc::clone(&count_ref);
            async move {
                count_ref.fetch_add(1, Ordering::Relaxed);
                let status = doc
                    .get_str("status")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "pending".to_string());
                // Continue processing other mails even if a single mail fails.
                if let Err(e) = process_mail(
                    &mails_ref,
                    &battle_reports_ref,
                    &duelbattle2_reports_ref,
                    &doc,
                )
                .await
                {
                    error!(error = %e, status = %status, "processing mail failed");
                    if let Err(e) = record_failure(&mails_ref, &doc, max_failures).await {
                        error!(error = %e, "failed to record mail processing failure");
                    }
                }
                Ok(())
            }
        })
        .await?;

    let processed = count.load(Ordering::Relaxed);
    if processed > 0 {
        info!(processed_count = processed, "processed mails");
    } else {
        debug!("no pending mails")
    }
    Ok(processed)
}

async fn record_failure(
    mails: &Collection<Document>,
    doc: &Document,
    max_failures: i64,
) -> Result<()> {
    let id = doc
        .get_object_id("_id")
        .with_context(|| "missing mail id")?;
    let now = DateTime::now();

    let opts = FindOneAndUpdateOptions::builder()
        .return_document(ReturnDocument::After)
        .build();
    let updated = mails
        .find_one_and_update(
            doc! { "_id": id, "status": { "$nin": ["processed", "error"] } },
            doc! {
                "$inc": { "metadata.processingFailures": 1 },
                "$set": { "metadata.lastFailureAt": now },
            },
        )
        .with_options(opts)
        .await?;

    let Some(updated) = updated else {
        return Ok(());
    };

    let failure_count = updated
        .get_document("metadata")
        .ok()
        .and_then(|metadata| metadata.get("processingFailures"))
        .and_then(|value| match value {
            Bson::Int32(v) => Some(i64::from(*v)),
            Bson::Int64(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(0);

    if failure_count >= max_failures {
        let update_result = mails
            .update_one(
                doc! { "_id": id, "status": { "$nin": ["processed", "error"] } },
                doc! { "$set": { "status": "error", "errorAt": now } },
            )
            .await?;
        if update_result.modified_count > 0 {
            info!(
                mail_id = %id,
                failure_count,
                "marked mail as error after repeated failures"
            );
        }
    }

    Ok(())
}

async fn process_mail(
    mails: &Collection<Document>,
    battle_reports: &Collection<Document>,
    duelbattle2_reports: &Collection<Document>,
    doc: &Document,
) -> Result<()> {
    let id = doc
        .get_object_id("_id")
        .with_context(|| "missing mail id")?;
    let mail = doc
        .get_document("mail")
        .with_context(|| "missing mail doc")?;
    let mail_hash = mail.get_str("hash").context("missing mail hash")?;
    let codec = mail.get_str("codec").unwrap_or("zstd");
    let status = doc.get_str("status").unwrap_or("pending");
    let previous_parent_hashes: Vec<String> = doc
        .get_document("metadata")
        .ok()
        .and_then(|metadata| metadata.get_array("previousHashes").ok())
        .map(|arr| {
            arr.iter()
                .filter_map(|value| value.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let raw = match mail.get("value") {
        Some(Bson::Binary(bin)) => &bin.bytes,
        _ => bail!("missing binary mail value"),
    };

    // Decompress
    let mail_bytes = match codec {
        "zstd" => zstd::decode_all(&raw[..])?,
        other => bail!("unsupported codec: {}", other),
    };

    // Convert to str
    let mail_str = std::str::from_utf8(&mail_bytes)?;

    // Parse mail metadata and detect type before selecting a processor.
    let decoded_mail: mail_decoder::Mail = serde_json::from_str(mail_str)?;
    let mail_type = mail_helper::detect_mail_type(&decoded_mail).context("missing mail type")?;
    let collection_label = match &mail_type {
        EmailType::Battle => "battleReports",
        EmailType::DuelBattle2 => "duelbattle2Reports",
        other => bail!("unsupported mail type: {:?}", other),
    };
    let target_collection = match &mail_type {
        EmailType::Battle => battle_reports,
        EmailType::DuelBattle2 => duelbattle2_reports,
        other => bail!("unsupported mail type: {:?}", other),
    };

    if status == "reprocess" {
        let parent_hashes_to_remove =
            parent_hashes_for_reprocess(&previous_parent_hashes, mail_hash);

        let delete_filter = doc! {
            "metadata.parentHash": { "$in": parent_hashes_to_remove.clone() }
        };
        let delete_result = target_collection.delete_many(delete_filter).await?;
        info!(
            mail_hash = %mail_hash,
            removed = delete_result.deleted_count,
            collection = collection_label,
            "removed reports for previous hashes"
        );
    }

    let now = DateTime::now();

    let docs = match mail_type {
        EmailType::Battle => {
            let mail_obj = mail_processor::process(mail_str)?;
            let mut docs = Vec::with_capacity(mail_obj.len());

            // Need to review bulk write API later, something might be wrong with it in current version of the driver
            for report in &mail_obj {
                let rep_json = serde_json::to_vec(report)?;
                let rep_hash = blake3_hash(&rep_json);

                let report_doc = bson::to_document(report)?;
                let mut metadata_doc = Document::new();
                metadata_doc.insert("hash", rep_hash);
                metadata_doc.insert("parentHash", mail_hash);

                let mut insert_doc = Document::new();
                insert_doc.insert("metadata", metadata_doc);
                insert_doc.insert("report", report_doc);
                insert_doc.insert("createdAt", now);

                docs.push(insert_doc);
            }

            docs
        }
        EmailType::DuelBattle2 => {
            let processed_mail =
                mail_processor_duelbattle2::process_sections(&decoded_mail.sections)?;
            let rep_json = serde_json::to_vec(&processed_mail)?;
            let rep_hash = blake3_hash(&rep_json);

            let report_doc = bson::to_document(&processed_mail)?;
            let mut metadata_doc = Document::new();
            metadata_doc.insert("hash", rep_hash);
            metadata_doc.insert("parentHash", mail_hash);

            let mut insert_doc = Document::new();
            insert_doc.insert("metadata", metadata_doc);
            insert_doc.insert("report", report_doc);
            insert_doc.insert("createdAt", now);

            vec![insert_doc]
        }
        other => bail!("unsupported mail type: {:?}", other),
    };

    if !docs.is_empty() {
        let opts = InsertManyOptions::builder().ordered(false).build();
        if let Err(e) = target_collection.insert_many(docs).with_options(opts).await {
            let is_dup_key = match e.kind.as_ref() {
                ErrorKind::BulkWrite(failure) => {
                    if failure.write_errors.is_empty() {
                        false
                    } else {
                        failure.write_errors.values().all(|c| c.code == 11000)
                    }
                }
                _ => false,
            };

            if !is_dup_key {
                return Err(e.into());
            }
        }
    }

    let mut update_doc = doc! {
        "$set": {
            "status": "processed",
            "processedAt": now,
        },
    };

    if status == "reprocess" {
        update_doc.insert("$unset", doc! { "metadata.previousHashes": "" });
    }

    mails
        .update_one(
            doc! { "_id": id, "status": { "$ne": "processed" } },
            update_doc,
        )
        .await?;

    Ok(())
}

fn parent_hashes_for_reprocess(previous_parent_hashes: &[String], mail_hash: &str) -> Vec<String> {
    let mut hashes = previous_parent_hashes.to_vec();

    if !hashes.iter().any(|hash| hash == mail_hash) {
        hashes.push(mail_hash.to_string());
    }

    hashes
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mongo_uri = std::env::var("MONGO_URI").expect("MONGO_URI environment variable must be set");
    let client = mongodb::Client::with_uri_str(&mongo_uri)
        .await
        .expect("failed to create MongoDB client");
    let db = client
        .default_database()
        .expect("MONGO_URI environment variable must include a database name");
    debug!(database = %db.name(), "connected to MongoDB");

    let batch_size = std::env::var("PROCESSOR_BATCH_SIZE")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(500);
    let concurrency = std::env::var("PROCESSOR_CONCURRENCY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(8);
    let max_failures = std::env::var("PROCESSOR_MAX_FAILURES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);
    let idle_sleep = std::env::var("PROCESSOR_IDLE_SLEEP_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(15));

    loop {
        match process(&db, batch_size, concurrency, max_failures).await {
            Ok(0) => {
                tokio::time::sleep(idle_sleep).await;
            }
            Ok(_) => {}
            Err(e) => {
                error!(error = %e, "processing failed");
                tokio::time::sleep(idle_sleep).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parent_hashes_for_reprocess;

    #[test]
    fn includes_current_mail_hash_when_missing() {
        let previous = vec!["prev1".to_string(), "prev2".to_string()];
        let result = parent_hashes_for_reprocess(&previous, "current");
        let expected = vec![
            "prev1".to_string(),
            "prev2".to_string(),
            "current".to_string(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn avoids_duplicate_mail_hash() {
        let previous = vec!["prev1".to_string(), "current".to_string()];
        let result = parent_hashes_for_reprocess(&previous, "current");
        let expected = vec!["prev1".to_string(), "current".to_string()];

        assert_eq!(result, expected);
    }
}
