use std::sync::Arc;

use futures::TryStreamExt;
use mongodb::bson::{Bson, Document, doc};
use mongodb::options::FindOptions;
use serde::Deserialize;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct MailMetadataDocument {
    #[serde(default)]
    pub mail_time: Option<Bson>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct LootEntryDocument {
    #[serde(default, rename = "type")]
    pub reward_type: Option<Bson>,
    #[serde(default)]
    pub sub_type: Option<Bson>,
    #[serde(default)]
    pub value: Option<Bson>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BattleNpcDocument {
    #[serde(default, rename = "type")]
    pub npc_type: Option<Bson>,
    #[serde(default)]
    pub b_type: Option<Bson>,
    #[serde(default)]
    pub loot: Option<Vec<LootEntryDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BattleOpponentDocument {
    #[serde(default)]
    pub player_id: Option<Bson>,
    #[serde(default)]
    pub npc: Option<BattleNpcDocument>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BattleMailDocument {
    #[serde(default)]
    pub metadata: Option<MailMetadataDocument>,
    #[serde(default)]
    pub opponents: Option<Vec<BattleOpponentDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BarbarianFortMailDocument {
    #[serde(default)]
    pub metadata: Option<MailMetadataDocument>,
    #[serde(default)]
    pub rewards: Option<Vec<LootEntryDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BaulurParticipantDocument {
    #[serde(default)]
    pub player_id: Option<Bson>,
    #[serde(default)]
    pub loot: Option<Vec<LootEntryDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BaulurMailDocument {
    #[serde(default)]
    pub metadata: Option<MailMetadataDocument>,
    #[serde(default)]
    pub participants: Option<Vec<BaulurParticipantDocument>>,
}

pub(crate) async fn fetch_barbarian_battle_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<BattleMailDocument>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "opponents.player_id": 1,
            "opponents.npc.type": 1,
            "opponents.npc.b_type": 1,
            "opponents.npc.loot": 1,
        })
        .build();

    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            { "opponents": { "$elemMatch": { "player_id": -2, "npc.b_type": 1 } } },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(state.reports_store.battle_collection(), filter, options).await
}

pub(crate) async fn fetch_barbarian_fort_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<BarbarianFortMailDocument>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "rewards": 1,
        })
        .build();
    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(
        state.reports_store.system_barbarian_fort_collection(),
        filter,
        options,
    )
    .await
}

pub(crate) async fn fetch_baulur_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    governor_id: i64,
    time_match: &Document,
) -> Result<Vec<BaulurMailDocument>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "participants.player_id": 1,
            "participants.loot": 1,
        })
        .build();
    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            { "participants": { "$elemMatch": { "player_id": governor_id } } },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(
        state.reports_store.barcanyonkillboss_collection(),
        filter,
        options,
    )
    .await
}

async fn fetch_collection_documents<T>(
    collection: &mongodb::Collection<Document>,
    filter: Document,
    options: FindOptions,
) -> Result<Vec<T>, ApiError>
where
    T: serde::de::DeserializeOwned + Send + Sync,
{
    let typed_collection = collection.clone_with_type::<T>();
    let cursor = typed_collection
        .find(filter)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    cursor
        .try_collect()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}
