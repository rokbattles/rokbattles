use std::time::Duration;

use mongodb::{
    Collection, IndexModel,
    bson::{DateTime, Document, doc},
    options::{IndexOptions, UpdateOptions},
};

const PROOF_LIFETIME_MILLIS: i64 = 120_000;

/// Error returned while recording or reading a DNS check proof.
#[derive(Debug, thiserror::Error)]
pub enum DnsCheckStoreError {
    #[error("database error: {0}")]
    Database(#[from] mongodb::error::Error),
}

/// Short-lived, anonymous proof store for resolver checks.
#[derive(Debug, Clone)]
pub struct DnsCheckStore {
    proofs: Collection<Document>,
}

impl DnsCheckStore {
    pub fn new(db: mongodb::Database) -> Self {
        Self { proofs: db.collection("dnsCheckProofs") }
    }

    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let indexes = [
            IndexModel::builder()
                .keys(doc! { "nonce": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "expiresAt": 1 })
                .options(IndexOptions::builder().expire_after(Some(Duration::ZERO)).build())
                .build(),
        ];
        for index in indexes {
            self.proofs.create_index(index).await?;
        }
        Ok(())
    }

    pub async fn mark(&self, nonce: &str) -> Result<(), DnsCheckStoreError> {
        let now = DateTime::now();
        let expires_at =
            DateTime::from_millis(now.timestamp_millis().saturating_add(PROOF_LIFETIME_MILLIS));
        self.proofs
            .update_one(
                doc! { "nonce": nonce },
                doc! {
                    "$set": { "expiresAt": expires_at },
                    "$setOnInsert": { "nonce": nonce, "createdAt": now },
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await?;
        Ok(())
    }

    pub async fn is_active(&self, nonce: &str) -> Result<bool, DnsCheckStoreError> {
        let proof = self
            .proofs
            .find_one(doc! { "nonce": nonce, "expiresAt": { "$gt": DateTime::now() } })
            .projection(doc! { "_id": 1 })
            .await?;
        Ok(proof.is_some())
    }
}
