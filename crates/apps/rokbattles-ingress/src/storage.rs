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

/// V2 raw binary mail metadata needed to compare a duplicate upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingCompressedRawMail {
    pub checksum: Option<String>,
    pub size: Option<usize>,
    pub algorithm: Option<String>,
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

    /// Load V2 comparison metadata, if this mail was already uploaded.
    pub async fn find_existing_compressed_raw(
        &self,
        mail_id: &str,
    ) -> mongodb::error::Result<Option<ExistingCompressedRawMail>> {
        let doc = self
            .compressed_raw
            .find_one(doc! { "mail.id": mail_id })
            .projection(doc! {
                "metadata.checksum": 1,
                "metadata.size": 1,
                "metadata.algo": 1,
            })
            .await?;
        Ok(doc.and_then(parse_existing_compressed_raw))
    }

    /// Load the compressed binary for an existing V2 mail.
    pub async fn find_compressed_raw_binary(
        &self,
        mail_id: &str,
        checksum: Option<&str>,
    ) -> mongodb::error::Result<Option<Vec<u8>>> {
        let doc = self
            .compressed_raw
            .find_one(compressed_raw_version_filter(mail_id, checksum))
            .projection(doc! { "mail.binary": 1 })
            .await?;
        Ok(doc.and_then(parse_compressed_raw_binary))
    }

    /// Insert a new V2 raw compressed mail document.
    pub async fn insert_compressed_raw(&self, doc: Document) -> mongodb::error::Result<()> {
        self.compressed_raw.insert_one(doc).await?;
        Ok(())
    }

    /// Replace the V2 document only if it is still the version the caller compared.
    pub async fn update_compressed_raw(
        &self,
        mail_id: &str,
        existing_checksum: Option<&str>,
        mut doc: Document,
    ) -> mongodb::error::Result<bool> {
        doc.remove("createdAt");
        let filter = compressed_raw_version_filter(mail_id, existing_checksum);
        let result = self.compressed_raw.update_one(filter, doc! { "$set": doc }).await?;
        Ok(result.modified_count == 1)
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
    let algorithm = metadata.and_then(|metadata| metadata.get_str("algo").ok()).map(str::to_string);
    Some(ExistingCompressedRawMail { checksum, size, algorithm })
}

fn parse_compressed_raw_binary(mut doc: Document) -> Option<Vec<u8>> {
    let mut mail = match doc.remove("mail")? {
        Bson::Document(mail) => mail,
        _ => return None,
    };
    match mail.remove("binary")? {
        Bson::Binary(binary) => Some(binary.bytes),
        _ => None,
    }
}

fn compressed_raw_version_filter(mail_id: &str, checksum: Option<&str>) -> Document {
    match checksum {
        Some(checksum) => doc! {
            "mail.id": mail_id,
            "metadata.checksum": checksum,
        },
        None => doc! {
            "mail.id": mail_id,
            "metadata.checksum": { "$exists": false },
        },
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Binary, spec::BinarySubtype};

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
                "algo": "zstd",
            },
        };
        let existing = parse_existing_compressed_raw(doc).expect("existing compressed mail");
        assert_eq!(
            existing,
            ExistingCompressedRawMail {
                checksum: Some("abc123".to_string()),
                size: Some(42),
                algorithm: Some("zstd".to_string()),
            }
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
    fn parses_compressed_raw_binary() {
        let doc = doc! {
            "mail": {
                "binary": Bson::Binary(Binary {
                    subtype: BinarySubtype::Generic,
                    bytes: vec![1, 2, 3],
                }),
            },
        };

        assert_eq!(parse_compressed_raw_binary(doc), Some(vec![1, 2, 3]));
    }

    #[test]
    fn compressed_raw_version_filter_matches_known_checksum() {
        let filter = compressed_raw_version_filter("mail-1", Some("old"));
        assert_eq!(
            filter,
            doc! {
                "mail.id": "mail-1",
                "metadata.checksum": "old",
            }
        );
    }

    #[test]
    fn compressed_raw_version_filter_matches_missing_checksum() {
        let filter = compressed_raw_version_filter("mail-1", None);
        assert_eq!(
            filter,
            doc! {
                "mail.id": "mail-1",
                "metadata.checksum": { "$exists": false },
            }
        );
    }
}
