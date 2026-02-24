//! MongoDB collection helpers for report routes and bind refresh jobs.

use mongodb::Collection;
use mongodb::IndexModel;
use mongodb::bson::{Document, doc};

/// Typed access to the collections this API needs.
#[derive(Debug, Clone)]
pub struct ReportsStore {
    mails_battle: Collection<Document>,
    mails_duelbattle2: Collection<Document>,
    claimed_governors: Collection<Document>,
}

impl ReportsStore {
    /// Build the store from a Mongo database handle.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            mails_battle: db.collection("mails_battle"),
            mails_duelbattle2: db.collection("mails_duelbattle2"),
            claimed_governors: db.collection("claimedGovernors"),
        }
    }

    /// Ensure indexes used by report filters, sorting, and bind refreshes exist.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let models = vec![
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.player_id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.player_id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.commanders.primary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.commanders.primary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_role": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.kvk": 1, "metadata.mail_time": -1 })
                .build(),
        ];

        for model in models {
            self.mails_battle.create_index(model).await?;
        }

        let duel_models = vec![
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.duel.team_id": 1, "metadata.mail_time": 1 })
                .build(),
        ];

        for model in duel_models {
            self.mails_duelbattle2.create_index(model).await?;
        }

        let claimed_governor_models = vec![
            IndexModel::builder().keys(doc! { "governorId": 1 }).build(),
            IndexModel::builder()
                .keys(doc! { "discordId": 1, "governorId": 1 })
                .build(),
        ];

        for model in claimed_governor_models {
            self.claimed_governors.create_index(model).await?;
        }

        Ok(())
    }

    /// Access the battle reports collection.
    pub fn battle_collection(&self) -> &Collection<Document> {
        &self.mails_battle
    }

    /// Access the Olympian Arena duel reports collection.
    pub fn duelbattle2_collection(&self) -> &Collection<Document> {
        &self.mails_duelbattle2
    }

    /// Access claimed governor bindings.
    pub fn claimed_governors_collection(&self) -> &Collection<Document> {
        &self.claimed_governors
    }
}
