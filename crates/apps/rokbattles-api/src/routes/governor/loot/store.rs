use std::sync::Arc;

use mongodb::{
    bson::{Bson, Document, doc},
    options::FindOptions,
};
use serde::Deserialize;

use crate::{
    db::exclude_test_client_filter, error::ApiError,
    routes::governor::store_utils::fetch_collection_documents, state::AppState,
};

const SYSTEM_BARBARIAN_FORT_SUB_TYPE: i32 = 11;
const BARBARIAN_FORT_SUB_PARAMS: [i32; 2] = [1, 4];
const MARAUDER_ENCAMPMENT_SUB_PARAMS: [i32; 1] = [3];

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
    pub experience: Option<Bson>,
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
    pub body: Option<BarbarianFortBodyDocument>,
    #[serde(default)]
    pub rewards: Option<Vec<LootEntryDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BarbarianFortBodyDocument {
    #[serde(default)]
    pub sub_param: Option<Bson>,
    #[serde(default)]
    pub content: Option<BarbarianFortContentDocument>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BarbarianFortContentDocument {
    #[serde(default)]
    pub level: Option<Bson>,
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
    pub npc: Option<BaulurNpcDocument>,
    #[serde(default)]
    pub participants: Option<Vec<BaulurParticipantDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct KaharTreasureMailDocument {
    #[serde(default)]
    pub metadata: Option<MailMetadataDocument>,
    #[serde(default)]
    pub loot: Option<Vec<LootEntryDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct KaruakCeremonyMailDocument {
    #[serde(default)]
    pub metadata: Option<MailMetadataDocument>,
    #[serde(default)]
    pub participants: Option<Vec<BaulurParticipantDocument>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct BaulurNpcDocument {
    #[serde(default, rename = "type")]
    pub npc_type: Option<Bson>,
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
            "opponents.npc.experience": 1,
            "opponents.npc.loot": 1,
        })
        .build();

    let filter = doc! {
        "$and": [
            exclude_test_client_filter(),
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
            "opponents.npc.experience": 1,
            "opponents.npc.loot": 1,
        })
        .build();

    let filter = doc! {
        "$and": [
            exclude_test_client_filter(),
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
    level: Option<i32>,
) -> Result<Vec<BarbarianFortMailDocument>, ApiError> {
    fetch_system_barbarian_fort_mails(
        state,
        mail_receiver,
        time_match,
        &BARBARIAN_FORT_SUB_PARAMS,
        level,
    )
    .await
}

pub(crate) async fn fetch_marauder_encampment_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
    level: Option<i32>,
) -> Result<Vec<BarbarianFortMailDocument>, ApiError> {
    fetch_system_barbarian_fort_mails(
        state,
        mail_receiver,
        time_match,
        &MARAUDER_ENCAMPMENT_SUB_PARAMS,
        level,
    )
    .await
}

async fn fetch_system_barbarian_fort_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
    sub_params: &[i32],
    level: Option<i32>,
) -> Result<Vec<BarbarianFortMailDocument>, ApiError> {
    let options = system_barbarian_fort_find_options();
    let filter = build_system_barbarian_fort_filter(mail_receiver, time_match, sub_params, level);

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
    npc_types: Option<&[i64]>,
    time_match: &Document,
) -> Result<Vec<BaulurMailDocument>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "npc.type": 1,
            "participants.player_id": 1,
            "participants.loot": 1,
        })
        .build();
    let mut clauses = vec![
        doc! { "metadata.mail_receiver": mail_receiver },
        doc! { "participants": { "$elemMatch": { "player_id": governor_id } } },
        time_match.clone(),
    ];
    if let Some(npc_types) = npc_types {
        clauses.push(doc! { "npc.type": { "$in": npc_types } });
    }
    let filter = doc! { "$and": clauses };

    fetch_collection_documents(state.reports_store.barcanyonkillboss_collection(), filter, options)
        .await
}

pub(crate) async fn fetch_kahar_treasure_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<KaharTreasureMailDocument>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "loot": 1,
        })
        .build();
    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(
        state.reports_store.system_kahar_treasure_collection(),
        filter,
        options,
    )
    .await
}

pub(crate) async fn fetch_karuak_ceremony_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    governor_id: i64,
    boss_id: i64,
    time_match: &Document,
) -> Result<Vec<KaruakCeremonyMailDocument>, ApiError> {
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
            { "boss.id": boss_id },
            { "participants": { "$elemMatch": { "player_id": governor_id } } },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(
        state.reports_store.event_member_loot_report_collection(),
        filter,
        options,
    )
    .await
}

fn system_barbarian_fort_find_options() -> FindOptions {
    FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "body.sub_param": 1,
            "body.content.level": 1,
            "rewards": 1,
        })
        .build()
}

fn build_system_barbarian_fort_filter(
    mail_receiver: &str,
    time_match: &Document,
    sub_params: &[i32],
    level: Option<i32>,
) -> Document {
    let mut clauses = vec![
        doc! { "metadata.mail_receiver": mail_receiver },
        doc! { "body.sub_type": SYSTEM_BARBARIAN_FORT_SUB_TYPE },
        doc! { "body.sub_param": { "$in": sub_params } },
        time_match.clone(),
    ];
    if let Some(level) = level {
        clauses.push(doc! { "body.content.level": level });
    }
    doc! { "$and": clauses }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Document;

    use super::*;

    #[test]
    fn barbarian_fort_filter_uses_sub_type_11_and_fort_sub_params() {
        let time_match = Document::new();

        let filter = build_system_barbarian_fort_filter(
            "player_42",
            &time_match,
            &BARBARIAN_FORT_SUB_PARAMS,
            None,
        );

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "metadata.mail_receiver": "player_42" },
                    { "body.sub_type": 11 },
                    { "body.sub_param": { "$in": [1, 4] } },
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
            &MARAUDER_ENCAMPMENT_SUB_PARAMS,
            None,
        );

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "metadata.mail_receiver": "player_42" },
                    { "body.sub_type": 11 },
                    { "body.sub_param": { "$in": [3] } },
                    {},
                ]
            }
        );
    }

    #[test]
    fn barbarian_fort_filter_includes_level_when_selected() {
        let time_match = Document::new();

        let filter = build_system_barbarian_fort_filter(
            "player_42",
            &time_match,
            &BARBARIAN_FORT_SUB_PARAMS,
            Some(11),
        );

        assert_eq!(
            filter,
            doc! {
                "$and": [
                    { "metadata.mail_receiver": "player_42" },
                    { "body.sub_type": 11 },
                    { "body.sub_param": { "$in": [1, 4] } },
                    {},
                    { "body.content.level": 11 },
                ]
            }
        );
    }
}
