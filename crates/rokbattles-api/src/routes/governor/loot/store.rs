use std::sync::Arc;

use mongodb::{
    bson::{Bson, Document, doc},
    options::FindOptions,
};
use serde::Deserialize;

use crate::{
    error::ApiError, routes::governor::store_utils::fetch_collection_documents, state::AppState,
};

const SYSTEM_BARBARIAN_FORT_SUB_TYPE: i32 = 11;
const BARBARIAN_FORT_SUB_PARAM: i32 = 1;
const MARAUDER_ENCAMPMENT_SUB_PARAM: i32 = 3;

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
            {
                "opponents": {
                    "$elemMatch": {
                        "player_id": -2,
                        "npc.b_type": 1,
                    }
                }
            },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(state.reports_store.battle_collection(), filter, options).await
}

pub(crate) async fn fetch_marauder_battle_mails(
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
            {
                "opponents": {
                    "$elemMatch": {
                        "player_id": -2,
                        "npc.type": { "$in": [99, 100] },
                        "npc.b_type": 15,
                    }
                }
            },
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
    fetch_system_barbarian_fort_mails(state, mail_receiver, time_match, BARBARIAN_FORT_SUB_PARAM)
        .await
}

pub(crate) async fn fetch_marauder_encampment_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<BarbarianFortMailDocument>, ApiError> {
    fetch_system_barbarian_fort_mails(
        state,
        mail_receiver,
        time_match,
        MARAUDER_ENCAMPMENT_SUB_PARAM,
    )
    .await
}

async fn fetch_system_barbarian_fort_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
    sub_param: i32,
) -> Result<Vec<BarbarianFortMailDocument>, ApiError> {
    let options = system_barbarian_fort_find_options();
    let filter = build_system_barbarian_fort_filter(mail_receiver, time_match, sub_param);

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

    fetch_collection_documents(state.reports_store.barcanyonkillboss_collection(), filter, options)
        .await
}

fn system_barbarian_fort_find_options() -> FindOptions {
    FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "rewards": 1,
        })
        .build()
}

fn build_system_barbarian_fort_filter(
    mail_receiver: &str,
    time_match: &Document,
    sub_param: i32,
) -> Document {
    doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            { "body.sub_type": SYSTEM_BARBARIAN_FORT_SUB_TYPE },
            { "body.sub_param": sub_param },
            time_match.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Document;

    use super::*;

    #[test]
    fn barbarian_fort_filter_uses_sub_type_11_and_sub_param_1() {
        let time_match = Document::new();

        let filter =
            build_system_barbarian_fort_filter("player_42", &time_match, BARBARIAN_FORT_SUB_PARAM);

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "metadata.mail_receiver": "player_42" },
                    { "body.sub_type": 11 },
                    { "body.sub_param": 1 },
                    {},
                ]
            }
        );
    }

    #[test]
    fn marauder_encampment_filter_uses_sub_type_11_and_sub_param_3() {
        let time_match = Document::new();

        let filter = build_system_barbarian_fort_filter(
            "player_42",
            &time_match,
            MARAUDER_ENCAMPMENT_SUB_PARAM,
        );

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "metadata.mail_receiver": "player_42" },
                    { "body.sub_type": 11 },
                    { "body.sub_param": 3 },
                    {},
                ]
            }
        );
    }
}
