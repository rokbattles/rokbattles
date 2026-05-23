//! MongoDB access for raw mail and temporary TCP stream batches.

use std::time::Duration;

use mongodb::{
    Collection, IndexModel,
    bson::{Bson, Document, doc},
    options::IndexOptions,
};

/// Ingress collections used by upload handlers.
#[derive(Debug, Clone)]
pub struct Storage {
    raw: Collection<Document>,
    raw_lossless: Collection<Document>,
    tcp_streams_raw: Collection<Document>,
}

/// Mail metadata needed to decide whether an upload is newer.
#[derive(Debug, Clone, Copy)]
pub struct ExistingMail {
    pub attack_count: i64,
}

impl Storage {
    /// Bind storage helpers to the configured database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            raw: db.collection("mails_raw"),
            raw_lossless: db.collection("mails_raw_lossless"),
            tcp_streams_raw: db.collection("tcp_streams_raw"),
        }
    }

    /// Create indexes used by the upload paths.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let mail_id_index = IndexModel::builder()
            .keys(doc! { "mail_id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        self.raw.create_index(mail_id_index.clone()).await?;
        self.raw_lossless.create_index(mail_id_index).await?;

        let tcp_batch_index = IndexModel::builder()
            .keys(doc! { "capture_id": 1, "batch_index": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();
        let tcp_created_index = IndexModel::builder()
            .keys(doc! { "createdAt": 1 })
            .options(
                IndexOptions::builder().expire_after(Duration::from_secs(60 * 60 * 24 * 7)).build(),
            )
            .build();
        let tcp_processor_index = IndexModel::builder()
            .keys(doc! { "status": 1, "stream_ended": 1, "updatedAt": 1 })
            .build();
        self.tcp_streams_raw.create_index(tcp_batch_index).await?;
        self.tcp_streams_raw.create_index(tcp_created_index).await?;
        self.tcp_streams_raw.create_index(tcp_processor_index).await?;
        Ok(())
    }

    /// Load existing mail metadata, if this mail was already uploaded.
    pub async fn find_existing(
        &self,
        mail_id: &str,
    ) -> mongodb::error::Result<Option<ExistingMail>> {
        let filter = doc! { "mail_id": mail_id };
        let doc = self
            .raw
            .find_one(filter)
            .projection(doc! { "mail_attack_count": 1, "createdAt": 1 })
            .await?;
        Ok(doc.and_then(parse_existing))
    }

    /// Insert a new raw mail document.
    pub async fn insert_raw(&self, doc: Document) -> mongodb::error::Result<()> {
        self.raw.insert_one(doc).await?;
        Ok(())
    }

    /// Update an existing raw mail document.
    pub async fn update_raw(&self, mail_id: &str, update: Document) -> mongodb::error::Result<()> {
        self.raw.update_one(doc! { "mail_id": mail_id }, doc! { "$set": update }).await?;
        Ok(())
    }

    /// Insert a new lossless mail document.
    pub async fn insert_lossless(&self, doc: Document) -> mongodb::error::Result<()> {
        self.raw_lossless.insert_one(doc).await?;
        Ok(())
    }

    /// Update an existing lossless mail document.
    pub async fn update_lossless(
        &self,
        mail_id: &str,
        update: Document,
    ) -> mongodb::error::Result<()> {
        self.raw_lossless.update_one(doc! { "mail_id": mail_id }, doc! { "$set": update }).await?;
        Ok(())
    }

    /// Store one raw TCP stream batch. Duplicate retries count as success.
    pub async fn upsert_tcp_stream_raw(
        &self,
        capture_id: &str,
        batch_index: i64,
        doc: Document,
    ) -> mongodb::error::Result<()> {
        self.tcp_streams_raw
            .update_one(
                doc! {
                    "capture_id": capture_id,
                    "batch_index": batch_index,
                },
                doc! { "$setOnInsert": doc },
            )
            .upsert(true)
            .await?;
        Ok(())
    }
}

fn parse_existing(doc: Document) -> Option<ExistingMail> {
    let attack_count = doc.get("mail_attack_count").and_then(bson_to_i64).unwrap_or(0);
    Some(ExistingMail { attack_count })
}

fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) => Some(*value as i64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_existing_mail() {
        let doc = doc! {
            "mail_attack_count": 7,
            "createdAt": mongodb::bson::DateTime::now(),
        };
        let existing = parse_existing(doc).expect("existing mail");
        assert_eq!(existing.attack_count, 7);
    }

    #[test]
    fn bson_to_i64_handles_numeric_variants() {
        assert_eq!(bson_to_i64(&Bson::Int32(5)), Some(5));
        assert_eq!(bson_to_i64(&Bson::Int64(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Double(3.7)), Some(3));
    }
}
