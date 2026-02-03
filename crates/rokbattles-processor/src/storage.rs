//! MongoDB access helpers for processor operations.

use mongodb::Collection;
use mongodb::Cursor;
use mongodb::IndexModel;
use mongodb::bson::{DateTime, Document, doc, oid::ObjectId};
use mongodb::options::{FindOptions, IndexOptions};

use crate::mail::MailType;

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_REPROCESS: &str = "reprocess";
pub const STATUS_PROCESSED: &str = "processed";

/// Typed access to raw and processed mail collections.
#[derive(Debug, Clone)]
pub struct Storage {
    raw: Collection<Document>,
    battle: Collection<Document>,
    duelbattle2: Collection<Document>,
    barcanyonkillboss: Collection<Document>,
}

impl Storage {
    /// Create storage helpers for the configured database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            raw: db.collection("mails_raw"),
            battle: db.collection(MailType::Battle.collection_name()),
            duelbattle2: db.collection(MailType::DuelBattle2.collection_name()),
            barcanyonkillboss: db.collection(MailType::BarCanyonKillBoss.collection_name()),
        }
    }

    /// Ensure required indexes exist.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let status_index = IndexModel::builder()
            .keys(doc! { "status": 1, "updatedAt": 1 })
            .build();
        self.raw.create_index(status_index).await?;

        let mail_id_index = IndexModel::builder()
            .keys(doc! { "metadata.mail_id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();
        self.battle.create_index(mail_id_index.clone()).await?;
        self.duelbattle2.create_index(mail_id_index.clone()).await?;
        self.barcanyonkillboss.create_index(mail_id_index).await?;

        Ok(())
    }

    /// Fetch a batch of pending or reprocess mail records.
    pub async fn find_pending(&self, batch_size: i64) -> mongodb::error::Result<Cursor<Document>> {
        let filter = doc! {
            "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] },
        };
        let opts = FindOptions::builder()
            .limit(batch_size)
            .sort(doc! { "updatedAt": 1 })
            .projection(doc! {
                "_id": 1,
                "mail_id": 1,
                "status": 1,
                "mail_value": 1,
            })
            .build();

        self.raw.find(filter).with_options(opts).await
    }

    /// Replace (or insert) the processed document for a mail.
    pub async fn upsert_processed(
        &self,
        mail_type: MailType,
        mail_id: &str,
        doc: Document,
    ) -> mongodb::error::Result<()> {
        let collection = match mail_type {
            MailType::Battle => &self.battle,
            MailType::DuelBattle2 => &self.duelbattle2,
            MailType::BarCanyonKillBoss => &self.barcanyonkillboss,
        };

        collection
            .replace_one(doc! { "metadata.mail_id": mail_id }, doc)
            .upsert(true)
            .await?;
        Ok(())
    }

    /// Mark a raw mail as processed.
    pub async fn mark_processed(&self, id: &ObjectId, now: DateTime) -> mongodb::error::Result<()> {
        self.raw
            .update_one(
                doc! { "_id": id, "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] } },
                doc! {
                    "$set": {
                        "status": STATUS_PROCESSED,
                        "processedAt": now,
                        "updatedAt": now,
                    }
                },
            )
            .await?;
        Ok(())
    }
}
