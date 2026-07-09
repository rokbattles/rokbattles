//! MongoDB access for raw mail uploads.

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
    compressed_raw: Collection<Document>,
}

/// Mail metadata needed to decide whether an upload is newer.
#[derive(Debug, Clone, Copy)]
pub struct ExistingMail {
    pub attack_count: i64,
}

/// V2 raw binary mail metadata needed for checksum dedupe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingCompressedRawMail {
    pub checksum: String,
}

impl Storage {
    /// Bind storage helpers to the configured database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            raw: db.collection("mails_raw"),
            raw_lossless: db.collection("mails_raw_lossless"),
            compressed_raw: db.collection("g_rok_mails"),
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

        let compressed_mail_id_index = IndexModel::builder()
            .keys(doc! { "mail.id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();
        let compressed_checksum_index =
            IndexModel::builder().keys(doc! { "metadata.checksum": 1 }).build();

        self.compressed_raw.create_index(compressed_mail_id_index).await?;
        self.compressed_raw.create_index(compressed_checksum_index).await?;

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

    /// Load V2 raw mail checksum metadata, if this mail was already uploaded.
    pub async fn find_existing_compressed_raw(
        &self,
        mail_id: &str,
    ) -> mongodb::error::Result<Option<ExistingCompressedRawMail>> {
        let doc = self
            .compressed_raw
            .find_one(doc! { "mail.id": mail_id })
            .projection(doc! { "metadata.checksum": 1 })
            .await?;
        Ok(doc.and_then(parse_existing_compressed_raw))
    }

    /// Insert a new V2 raw compressed mail document.
    pub async fn insert_compressed_raw(&self, doc: Document) -> mongodb::error::Result<()> {
        self.compressed_raw.insert_one(doc).await?;
        Ok(())
    }

    /// Replace the mutable fields of an existing V2 raw compressed mail document.
    pub async fn update_compressed_raw(
        &self,
        mail_id: &str,
        mut doc: Document,
    ) -> mongodb::error::Result<()> {
        doc.remove("createdAt");
        self.compressed_raw.update_one(doc! { "mail.id": mail_id }, doc! { "$set": doc }).await?;
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
}

fn parse_existing(doc: Document) -> Option<ExistingMail> {
    let attack_count = doc.get("mail_attack_count").and_then(bson_to_i64).unwrap_or(0);
    Some(ExistingMail { attack_count })
}

fn parse_existing_compressed_raw(doc: Document) -> Option<ExistingCompressedRawMail> {
    let checksum = doc.get_document("metadata").ok()?.get_str("checksum").ok()?.to_string();
    Some(ExistingCompressedRawMail { checksum })
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
    fn parses_existing_compressed_raw_mail() {
        let doc = doc! {
            "metadata": {
                "checksum": "abc123",
            },
        };
        let existing = parse_existing_compressed_raw(doc).expect("existing compressed mail");
        assert_eq!(existing.checksum, "abc123");
    }

    #[test]
    fn bson_to_i64_handles_numeric_variants() {
        assert_eq!(bson_to_i64(&Bson::Int32(5)), Some(5));
        assert_eq!(bson_to_i64(&Bson::Int64(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Double(3.7)), Some(3));
    }
}
