//! MongoDB persistence helpers for raw and lossless mail storage.

use mongodb::Collection;
use mongodb::IndexModel;
use mongodb::bson::{Bson, Document, doc};
use mongodb::options::IndexOptions;

/// Typed access to the mail collections.
#[derive(Debug, Clone)]
pub struct Storage {
    raw: Collection<Document>,
    raw_lossless: Collection<Document>,
}

/// Snapshot of the existing mail metadata.
#[derive(Debug, Clone, Copy)]
pub struct ExistingMail {
    pub attack_count: i64,
}

impl Storage {
    /// Create storage helpers for the configured database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            raw: db.collection("mails_raw"),
            raw_lossless: db.collection("mails_raw_lossless"),
        }
    }

    /// Ensure required indexes exist.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let mail_id_index = IndexModel::builder()
            .keys(doc! { "mail_id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        self.raw.create_index(mail_id_index.clone()).await?;
        self.raw_lossless.create_index(mail_id_index).await?;
        Ok(())
    }

    /// Load the existing mail metadata if present.
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
        self.raw
            .update_one(doc! { "mail_id": mail_id }, doc! { "$set": update })
            .await?;
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
        self.raw_lossless
            .update_one(doc! { "mail_id": mail_id }, doc! { "$set": update })
            .await?;
        Ok(())
    }
}

fn parse_existing(doc: Document) -> Option<ExistingMail> {
    let attack_count = doc
        .get("mail_attack_count")
        .and_then(bson_to_i64)
        .unwrap_or(0);
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
