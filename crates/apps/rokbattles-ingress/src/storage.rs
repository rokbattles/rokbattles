//! MongoDB access for raw mail uploads.

use mongodb::{
    Collection, IndexModel,
    bson::{Document, doc},
    error::{ErrorKind, WriteFailure},
    options::IndexOptions,
};

/// Ingress collections used by upload handlers.
#[derive(Debug, Clone)]
pub struct Storage {
    compressed_raw: Collection<Document>,
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

    /// Check whether a mail was uploaded, regardless of its metadata or processing status.
    pub async fn compressed_raw_exists(&self, mail_id: &str) -> mongodb::error::Result<bool> {
        let doc = self
            .compressed_raw
            .find_one(doc! { "mail.id": mail_id })
            .projection(doc! { "_id": 1 })
            .await?;

        Ok(doc.is_some())
    }

    /// Insert an immutable raw mail; return false if another upload already stored its ID.
    pub async fn insert_compressed_raw(
        &self,
        mail_id: &str,
        doc: Document,
    ) -> mongodb::error::Result<bool> {
        match self.compressed_raw.insert_one(doc).await {
            Ok(_) => Ok(true),
            Err(error) => {
                // The unique mail.id index also protects uploads that race the existence check.
                if matches!(
                    error.kind.as_ref(),
                    ErrorKind::Write(WriteFailure::WriteError(write_error))
                        if write_error.code == 11000
                ) && self.compressed_raw_exists(mail_id).await?
                {
                    return Ok(false);
                }

                Err(error)
            }
        }
    }
}

fn source_mail_id_index() -> IndexModel {
    IndexModel::builder()
        .keys(doc! { "mail.id": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build()
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
}
