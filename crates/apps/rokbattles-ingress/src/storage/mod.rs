//! MongoDB access for raw mail uploads.
//!
//! The unique `mail.id` index prevents duplicate IDs during concurrent inserts.
//! Ingress inserts new documents but never updates existing mail.

pub(crate) mod document;

use mongodb::{
    Collection, IndexModel,
    bson::{Document, doc},
    error::{ErrorKind, WriteFailure},
    options::IndexOptions,
};

/// Access to the `g_rok_mails` collection.
#[derive(Debug, Clone)]
pub struct Storage {
    compressed_raw: Collection<Document>,
}

impl Storage {
    /// Creates a handle to the mail collection in `db`.
    pub fn new(db: mongodb::Database) -> Self {
        Self { compressed_raw: db.collection("g_rok_mails") }
    }

    /// Ensures the unique mail-ID index exists before serving requests.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.compressed_raw.create_index(source_mail_id_index()).await?;

        Ok(())
    }

    /// Checks for a stored mail ID, regardless of metadata or processing status.
    pub async fn compressed_raw_exists(&self, mail_id: &str) -> mongodb::error::Result<bool> {
        let doc = self
            .compressed_raw
            .find_one(doc! { "mail.id": mail_id })
            .projection(doc! { "_id": 1 })
            .await?;

        Ok(doc.is_some())
    }

    /// Inserts a raw mail document, returning `false` if its ID is already stored.
    ///
    /// Requires [`Self::ensure_indexes`] to have succeeded and `mail_id` to match
    /// the document's `mail.id`. Other database errors are propagated.
    pub async fn insert_compressed_raw(
        &self,
        mail_id: &str,
        doc: Document,
    ) -> mongodb::error::Result<bool> {
        match self.compressed_raw.insert_one(doc).await {
            Ok(_) => Ok(true),
            Err(error) => {
                // Another upload may have inserted this ID after the existence check.
                // Confirm the ID exists so an unrelated duplicate-key error is not skipped.
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
