//! MongoDB access helpers for processor operations.

use mail_registry::MailType;
use mongodb::{
    Collection, Cursor, IndexModel,
    bson::{DateTime, Document, doc, oid::ObjectId},
    options::{FindOptions, IndexOptions},
};

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_REPROCESS: &str = "reprocess";
pub const STATUS_PROCESSED: &str = "processed";
pub const STATUS_ERROR: &str = "error";

/// Typed access to raw and processed mail collections.
#[derive(Debug, Clone)]
pub struct Storage {
    raw: Collection<Document>,
    battle: Collection<Document>,
    duelbattle2: Collection<Document>,
    barcanyonkillboss: Collection<Document>,
    eventmemberlootreport: Collection<Document>,
    rss: Collection<Document>,
    system_barbarianfort: Collection<Document>,
    system_kahartreasure: Collection<Document>,
    alliance_aoobattleresults: Collection<Document>,
    alliance_aoobattleinfo: Collection<Document>,
    alliance_aooindividualresults: Collection<Document>,
    alliance_aooregistration: Collection<Document>,
    scoutreport: Collection<Document>,
}

impl Storage {
    /// Create storage helpers for the configured database.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            raw: db.collection("mails_raw"),
            battle: db.collection(MailType::Battle.collection_name()),
            duelbattle2: db.collection(MailType::DuelBattle2.collection_name()),
            barcanyonkillboss: db.collection(MailType::BarCanyonKillBoss.collection_name()),
            eventmemberlootreport: db.collection(MailType::EventMemberLootReport.collection_name()),
            rss: db.collection(MailType::Rss.collection_name()),
            system_barbarianfort: db.collection(MailType::SystemBarbarianFort.collection_name()),
            system_kahartreasure: db.collection(MailType::SystemKaharTreasure.collection_name()),
            alliance_aoobattleresults: db
                .collection(MailType::AllianceAOOBattleResults.collection_name()),
            alliance_aoobattleinfo: db
                .collection(MailType::AllianceAOOBattleInfo.collection_name()),
            alliance_aooindividualresults: db
                .collection(MailType::AllianceAOOIndividualResults.collection_name()),
            alliance_aooregistration: db
                .collection(MailType::AllianceAOORegistration.collection_name()),
            scoutreport: db.collection(MailType::ScoutReport.collection_name()),
        }
    }

    /// Ensure required indexes exist.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let status_index = IndexModel::builder().keys(doc! { "status": 1, "updatedAt": 1 }).build();
        self.raw.create_index(status_index).await?;

        let mail_id_index = IndexModel::builder()
            .keys(doc! { "metadata.mail_id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();
        self.battle.create_index(mail_id_index.clone()).await?;
        self.duelbattle2.create_index(mail_id_index.clone()).await?;
        self.barcanyonkillboss.create_index(mail_id_index.clone()).await?;
        self.eventmemberlootreport.create_index(mail_id_index.clone()).await?;
        self.rss.create_index(mail_id_index.clone()).await?;
        self.system_barbarianfort.create_index(mail_id_index.clone()).await?;
        self.system_kahartreasure.create_index(mail_id_index.clone()).await?;
        self.alliance_aoobattleresults.create_index(mail_id_index.clone()).await?;
        self.alliance_aoobattleinfo.create_index(mail_id_index.clone()).await?;
        self.alliance_aooindividualresults.create_index(mail_id_index.clone()).await?;
        self.alliance_aooregistration.create_index(mail_id_index.clone()).await?;
        self.scoutreport.create_index(mail_id_index).await?;

        Ok(())
    }

    /// Fetch a batch of pending or reprocess mail records.
    pub async fn find_pending(&self, batch_size: i64) -> mongodb::error::Result<Cursor<Document>> {
        let filter = doc! {
            "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] },
        };
        let opts = pending_find_options(batch_size);

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
            MailType::EventMemberLootReport => &self.eventmemberlootreport,
            MailType::Rss => &self.rss,
            MailType::SystemBarbarianFort => &self.system_barbarianfort,
            MailType::SystemKaharTreasure => &self.system_kahartreasure,
            MailType::AllianceAOOBattleResults => &self.alliance_aoobattleresults,
            MailType::AllianceAOOBattleInfo => &self.alliance_aoobattleinfo,
            MailType::AllianceAOOIndividualResults => &self.alliance_aooindividualresults,
            MailType::AllianceAOORegistration => &self.alliance_aooregistration,
            MailType::ScoutReport => &self.scoutreport,
        };

        collection.replace_one(doc! { "metadata.mail_id": mail_id }, doc).upsert(true).await?;
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

    /// Mark a raw mail as failed to prevent indefinite retries.
    pub async fn mark_error(&self, id: &ObjectId, now: DateTime) -> mongodb::error::Result<()> {
        self.raw
            .update_one(
                doc! { "_id": id, "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] } },
                doc! {
                    "$set": {
                        "status": STATUS_ERROR,
                        "updatedAt": now,
                    }
                },
            )
            .await?;
        Ok(())
    }
}

fn pending_find_options(batch_size: i64) -> FindOptions {
    FindOptions::builder()
        .limit(batch_size)
        .sort(doc! { "status": 1, "updatedAt": 1 })
        .projection(doc! {
            "_id": 1,
            "mail_id": 1,
            "status": 1,
            "mail_value": 1,
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_mail_is_sorted_before_reprocess_mail() {
        assert!(STATUS_PENDING < STATUS_REPROCESS);

        let options = pending_find_options(100);

        assert_eq!(options.limit, Some(100));
        assert_eq!(options.sort, Some(doc! { "status": 1, "updatedAt": 1 }));
    }
}
