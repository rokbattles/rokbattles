//! MongoDB access helpers for processor operations.

use mail_registry::MailType;
use mongodb::{
    Collection, Cursor, IndexModel,
    bson::{Bson, DateTime, Document, doc, oid::ObjectId},
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
            raw: db.collection("g_rok_mails"),
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
        source_checksum: &str,
        source_size: i64,
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

        let update = processed_update_pipeline(source_checksum, source_size, doc);
        collection.update_one(doc! { "metadata.mail_id": mail_id }, update).upsert(true).await?;
        Ok(())
    }

    /// Mark a raw mail version as processed if it is still current.
    pub async fn mark_processed(
        &self,
        id: &ObjectId,
        checksum: &str,
        size: i64,
        now: DateTime,
    ) -> mongodb::error::Result<bool> {
        let result = self
            .raw
            .update_one(
                current_version_filter(id, checksum, size),
                doc! {
                    "$set": {
                        "status": STATUS_PROCESSED,
                        "processedAt": now,
                        "updatedAt": now,
                    }
                },
            )
            .await?;
        Ok(result.modified_count == 1)
    }

    /// Mark a raw mail as failed only if the observed source fields are unchanged.
    pub async fn mark_error(
        &self,
        id: &ObjectId,
        checksum: Option<&Bson>,
        size: Option<&Bson>,
        now: DateTime,
    ) -> mongodb::error::Result<bool> {
        let result = self
            .raw
            .update_one(
                observed_version_filter(id, checksum, size),
                doc! {
                    "$set": {
                        "status": STATUS_ERROR,
                        "updatedAt": now,
                    }
                },
            )
            .await?;
        Ok(result.modified_count == 1)
    }
}

fn pending_find_options(batch_size: i64) -> FindOptions {
    FindOptions::builder()
        .limit(batch_size)
        .sort(doc! { "status": 1, "updatedAt": 1 })
        .projection(doc! {
            "_id": 1,
            "status": 1,
            "metadata.checksum": 1,
            "metadata.size": 1,
            "metadata.algo": 1,
            "mail.id": 1,
            "mail.binary": 1,
        })
        .build()
}

fn current_version_filter(id: &ObjectId, checksum: &str, size: i64) -> Document {
    doc! {
        "_id": id,
        "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] },
        "metadata.checksum": checksum,
        "metadata.size": size,
    }
}

fn observed_version_filter(
    id: &ObjectId,
    checksum: Option<&Bson>,
    size: Option<&Bson>,
) -> Document {
    let mut filter = doc! {
        "_id": id,
        "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] },
    };
    insert_observed_field(&mut filter, "metadata.checksum", checksum);
    insert_observed_field(&mut filter, "metadata.size", size);
    filter
}

fn insert_observed_field(filter: &mut Document, field: &str, value: Option<&Bson>) {
    filter.insert(field, value.cloned().unwrap_or_else(|| doc! { "$exists": false }.into()));
}

fn processed_update_pipeline(
    source_checksum: &str,
    source_size: i64,
    processed: Document,
) -> Vec<Document> {
    vec![doc! {
        "$replaceWith": {
            "$cond": {
                "if": {
                    "$or": [
                        {
                            "$lt": [
                                { "$ifNull": ["$metadata.source_size", -1_i64] },
                                source_size,
                            ]
                        },
                        {
                            "$and": [
                                { "$eq": ["$metadata.source_size", source_size] },
                                { "$eq": ["$metadata.source_checksum", source_checksum] },
                            ]
                        },
                    ]
                },
                "then": { "$literal": Bson::Document(processed) },
                "else": "$$ROOT",
            }
        }
    }]
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

    #[test]
    fn current_version_filter_guards_checksum_and_size() {
        let id = ObjectId::new();
        let filter = current_version_filter(&id, "checksum", 42);
        assert_eq!(
            filter,
            doc! {
                "_id": id,
                "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] },
                "metadata.checksum": "checksum",
                "metadata.size": 42_i64,
            }
        );
    }

    #[test]
    fn observed_version_filter_matches_missing_fields_exactly() {
        let id = ObjectId::new();
        let filter = observed_version_filter(&id, None, None);
        assert_eq!(
            filter,
            doc! {
                "_id": id,
                "status": { "$in": [STATUS_PENDING, STATUS_REPROCESS] },
                "metadata.checksum": { "$exists": false },
                "metadata.size": { "$exists": false },
            }
        );
    }

    #[test]
    fn processed_update_pipeline_replaces_older_or_matching_output() {
        let processed = doc! {
            "metadata": {
                "mail_id": "mail-1",
                "source_size": 42_i64,
            }
        };
        let pipeline = processed_update_pipeline("checksum", 42, processed.clone());
        assert_eq!(
            pipeline,
            vec![doc! {
                "$replaceWith": {
                    "$cond": {
                        "if": {
                            "$or": [
                                {
                                    "$lt": [
                                        { "$ifNull": ["$metadata.source_size", -1_i64] },
                                        42_i64,
                                    ]
                                },
                                {
                                    "$and": [
                                        { "$eq": ["$metadata.source_size", 42_i64] },
                                        { "$eq": ["$metadata.source_checksum", "checksum"] },
                                    ]
                                },
                            ]
                        },
                        "then": { "$literal": Bson::Document(processed) },
                        "else": "$$ROOT",
                    }
                }
            }]
        );
    }
}
