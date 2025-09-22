use anyhow::{Context, Result, bail};
use blake3::Hasher;
use futures::stream::TryStreamExt;
use mongodb::{
    Collection, Database, bson,
    bson::Bson,
    bson::DateTime,
    bson::{Document, doc},
    error::ErrorKind,
    options::FindOptions,
    options::InsertManyOptions,
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

async fn process(db: &Database, cutoff_microseconds: i64) -> Result<()> {
    let mails = db.collection::<Document>("mails");
    let battle_reports = db.collection::<Document>("battleReports");

    let filter = doc! {
        "status": { "$in": ["pending", "reprocess"] },
        "mail.time": { "$gte": cutoff_microseconds }
    };

    // 100 raw reports at a time
    let opts = FindOptions::builder()
        .limit(100)
        .sort(doc! { "mail.time": 1 })
        .projection(doc! {
            "_id": 1,
            "mail.hash": 1,
            "mail.codec": 1,
            "mail.value": 1,
        })
        .build();

    let mut cursor = mails.find(filter).with_options(opts).await?;
    let mut count = 0usize;

    while let Some(doc) = cursor.try_next().await? {
        count += 1;
        let status = doc
            .get_str("status")
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "pending".to_string());
        if let Err(e) = process_mail(&mails, &battle_reports, &doc).await {
            error!(error = %e, status = %status, "processing mail failed");
        }
    }

    if count > 0 {
        info!(processed_count = count, "processed mails");
    } else {
        debug!("no pending mails")
    }
    Ok(())
}

async fn process_mail(
    mails: &Collection<Document>,
    battle_reports: &Collection<Document>,
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

    if status == "reprocess" {
        let parent_hashes_to_remove =
            parent_hashes_for_reprocess(&previous_parent_hashes, mail_hash);

        let delete_filter = doc! {
            "metadata.parentHash": { "$in": parent_hashes_to_remove.clone() }
        };
        let delete_result = battle_reports.delete_many(delete_filter).await?;
        info!(
            mail_hash = %mail_hash,
            removed = delete_result.deleted_count,
            "removed battle reports for previous hashes"
        );
    }

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

    // Parse mail
    let mail_obj = mail_processor::process(mail_str)?;
    let now = DateTime::now();

    let mut docs: Vec<Document> = Vec::with_capacity(mail_obj.len());

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

    if !docs.is_empty() {
        let opts = InsertManyOptions::builder().ordered(false).build();
        if let Err(e) = battle_reports.insert_many(docs).with_options(opts).await {
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

    // We'll process older reports over time, but first few days or week, we're only processing newer reports
    // January 1st 2025 00:00 UTC
    let cutoff_microseconds: i64 = 1735689600000000;

    // 15 second interval
    let mut tick = tokio::time::interval(Duration::from_secs(15));
    loop {
        tick.tick().await;
        if let Err(e) = process(&db, cutoff_microseconds).await {
            error!(error = %e, "processing failed");
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
