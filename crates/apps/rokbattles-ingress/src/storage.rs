//! MongoDB access for raw mail uploads.

use mongodb::{
    Collection, IndexModel,
    bson::{Bson, Document, doc},
    options::IndexOptions,
};

/// Ingress collections used by upload handlers.
#[derive(Debug, Clone)]
pub struct Storage {
    compressed_raw: Collection<Document>,
}

/// V2 raw binary mail metadata needed to decide whether an upload is larger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingCompressedRawMail {
    pub checksum: Option<String>,
    pub size: Option<usize>,
}

impl Storage {
    /// Bind storage helpers to the configured database.
    pub fn new(db: mongodb::Database) -> Self {
        Self { compressed_raw: db.collection("g_rok_mails") }
    }

    /// Create indexes used by the upload paths.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.compressed_raw.create_index(source_mail_id_index()).await?;

        Ok(())
    }

    /// Load V2 checksum and uncompressed size metadata, if this mail was already uploaded.
    pub async fn find_existing_compressed_raw(
        &self,
        mail_id: &str,
    ) -> mongodb::error::Result<Option<ExistingCompressedRawMail>> {
        let doc = self
            .compressed_raw
            .find_one(doc! { "mail.id": mail_id })
            .projection(doc! { "metadata.checksum": 1, "metadata.size": 1 })
            .await?;
        Ok(doc.and_then(parse_existing_compressed_raw))
    }

    /// Insert a new V2 raw compressed mail document.
    pub async fn insert_compressed_raw(&self, doc: Document) -> mongodb::error::Result<()> {
        self.compressed_raw.insert_one(doc).await?;
        Ok(())
    }

    /// Replace an existing V2 document only when its checksum differs and stored size is smaller.
    pub async fn update_compressed_raw(
        &self,
        mail_id: &str,
        checksum: &str,
        size: i64,
        mut doc: Document,
    ) -> mongodb::error::Result<()> {
        doc.remove("createdAt");
        let filter = compressed_raw_update_filter(mail_id, checksum, size);
        self.compressed_raw.update_one(filter, doc! { "$set": doc }).await?;
        Ok(())
    }
}

fn source_mail_id_index() -> IndexModel {
    IndexModel::builder()
        .keys(doc! { "mail.id": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build()
}

fn parse_existing_compressed_raw(doc: Document) -> Option<ExistingCompressedRawMail> {
    let metadata = doc.get_document("metadata").ok();
    let checksum =
        metadata.and_then(|metadata| metadata.get_str("checksum").ok()).map(str::to_string);
    let size = metadata
        .and_then(|metadata| metadata.get("size"))
        .and_then(|size| match size {
            Bson::Int32(size) => Some(i64::from(*size)),
            Bson::Int64(size) => Some(*size),
            _ => None,
        })
        .and_then(|size| usize::try_from(size).ok());
    Some(ExistingCompressedRawMail { checksum, size })
}

fn compressed_raw_update_filter(mail_id: &str, checksum: &str, size: i64) -> Document {
    doc! {
        "mail.id": mail_id,
        "metadata.checksum": { "$ne": checksum },
        "metadata.size": { "$lt": size },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_mail_id_index_is_unique() {
        let index = source_mail_id_index();
        assert_eq!(index.keys, doc! { "mail.id": 1 });
        assert_eq!(index.options.and_then(|options| options.unique), Some(true));
    }

    #[test]
    fn parses_existing_compressed_raw_mail() {
        let doc = doc! {
            "metadata": {
                "checksum": "abc123",
                "size": 42_i64,
            },
        };
        let existing = parse_existing_compressed_raw(doc).expect("existing compressed mail");
        assert_eq!(
            existing,
            ExistingCompressedRawMail { checksum: Some("abc123".to_string()), size: Some(42) }
        );
    }

    #[test]
    fn parses_existing_compressed_raw_mail_with_missing_size() {
        let doc = doc! { "metadata": { "checksum": "abc123" } };
        let existing = parse_existing_compressed_raw(doc).expect("existing compressed mail");
        assert_eq!(existing.size, None);
    }

    #[test]
    fn parses_existing_compressed_raw_mail_with_invalid_size() {
        let doc = doc! { "metadata": { "checksum": "abc123", "size": 42.5 } };
        let existing = parse_existing_compressed_raw(doc).expect("existing compressed mail");
        assert_eq!(existing.size, None);
    }

    #[test]
    fn compressed_raw_update_filter_requires_different_checksum_and_smaller_stored_size() {
        let filter = compressed_raw_update_filter("mail-1", "new", 100);
        assert_eq!(
            filter,
            doc! {
                "mail.id": "mail-1",
                "metadata.checksum": { "$ne": "new" },
                "metadata.size": { "$lt": 100_i64 },
            }
        );
    }
}
