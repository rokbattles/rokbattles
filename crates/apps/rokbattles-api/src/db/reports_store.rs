//! MongoDB collection helpers shared by API routes and jobs.

use mongodb::{
    Collection, IndexModel,
    bson::{Bson, Document, doc},
    options::IndexOptions,
};

/// App ID used by the internal test client.
pub const TEST_CLIENT_APP_ID: i64 = 10_088_010;

/// Exclude reports that match the internal test client.
pub fn exclude_test_client_filter() -> Document {
    doc! { "sender.app_id": { "$ne": TEST_CLIENT_APP_ID } }
}

/// Mongo collections
#[derive(Debug, Clone)]
pub struct ReportsStore {
    mails_battle: Collection<Document>,
    mails_duelbattle2: Collection<Document>,
    mails_alliance_aoobattleresults: Collection<Document>,
    mails_alliance_aoobattleinfo: Collection<Document>,
    mails_alliance_aooindividualresults: Collection<Document>,
    mails_alliance_aooregistration: Collection<Document>,
    mails_system_barbarianfort: Collection<Document>,
    mails_system_kahartreasure: Collection<Document>,
    mails_eventmemberlootreport: Collection<Document>,
    mails_barcanyonkillboss: Collection<Document>,
    mails_rss: Collection<Document>,
    claimed_governors: Collection<Document>,
    g_rok_prec_barbarian: Collection<Document>,
    g_rok_prec_barbarianfort: Collection<Document>,
    g_rok_prec_baulur: Collection<Document>,
    g_rok_prec_kahartreasure: Collection<Document>,
    g_rok_prec_karuakceremony: Collection<Document>,
    g_rok_prec_drastc: Collection<Document>,
    g_rok_prec_cmdr_pairings_v2: Collection<Document>,
}

impl ReportsStore {
    /// Create the store from a Mongo database handle.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            mails_battle: db.collection("mails_battle"),
            mails_duelbattle2: db.collection("mails_duelbattle2"),
            mails_alliance_aoobattleresults: db.collection("mails_alliance_aoobattleresults"),
            mails_alliance_aoobattleinfo: db.collection("mails_alliance_aoobattleinfo"),
            mails_alliance_aooindividualresults: db
                .collection("mails_alliance_aooindividualresults"),
            mails_alliance_aooregistration: db.collection("mails_alliance_aooregistration"),
            mails_system_barbarianfort: db.collection("mails_system_barbarianfort"),
            mails_system_kahartreasure: db.collection("mails_system_kahartreasure"),
            mails_eventmemberlootreport: db.collection("mails_eventmemberlootreport"),
            mails_barcanyonkillboss: db.collection("mails_barcanyonkillboss"),
            mails_rss: db.collection("mails_rss"),
            claimed_governors: db.collection("claimedGovernors"),
            g_rok_prec_barbarian: db.collection("g_rok_prec_barbarian"),
            g_rok_prec_barbarianfort: db.collection("g_rok_prec_barbarianfort"),
            g_rok_prec_baulur: db.collection("g_rok_prec_baulur"),
            g_rok_prec_kahartreasure: db.collection("g_rok_prec_kahartreasure"),
            g_rok_prec_karuakceremony: db.collection("g_rok_prec_karuakceremony"),
            g_rok_prec_drastc: db.collection("g_rok_prec_drastc"),
            g_rok_prec_cmdr_pairings_v2: db.collection("g_rok_prec_cmdr_pairings_v2"),
        }
    }

    /// Ensure indexes for report filters, sorting, and bind refresh are in place.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        let models = vec![
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.player_id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.player_id": 1, "sender.commanders.primary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.player_id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.player_id": 1, "opponents.npc.type": 1, "opponents.npc.b_type": 1 })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_receiver": 1,
                    "opponents.player_id": 1,
                    "opponents.npc.b_type": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_receiver": 1,
                    "opponents.player_id": 1,
                    "opponents.npc.b_type": 1,
                    "opponents.npc.type": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.commanders.primary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "sender.commanders.secondary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "sender.commanders.primary.id": 1,
                    "sender.commanders.secondary.id": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.kvk": 1,
                    "sender.commanders.primary.id": 1,
                    "sender.commanders.secondary.id": 1,
                })
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.commanders.primary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.commanders.secondary.id": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "opponents.commanders.primary.id": 1,
                    "opponents.commanders.secondary.id": 1,
                    "opponents.player_id": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
            // IndexModel::builder()
            //     .keys(doc! {
            //         "metadata.kvk": 1,
            //         "opponents.commanders.primary.id": 1,
            //         "opponents.commanders.secondary.id": 1,
            //         "opponents.player_id": 1,
            //     })
            //     .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_role": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.kvk": 1, "metadata.mail_time": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_time": -1,
                    "metadata.kvk": 1,
                    "opponents.player_id": 1,
                })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! {
                            "metadata.kvk": true,
                            "opponents.player_id": { "$gt": 0 },
                        })
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1, "metadata.kvk": 1 })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! {
                            "$and": [
                                {
                                    "opponents.player_id": { "$gt": 0 }
                                },
                                { "metadata.kvk": false },
                                {
                                    "$or": [
                                        { "metadata.mail_role": { "$lt": "dungeon" } },
                                        { "metadata.mail_role": { "$gt": "dungeon" } },
                                    ]
                                },
                                {
                                    "$or": [
                                        { "sender.supreme_strife.battle_id": { "$in": [Bson::Null, Bson::String(String::new())] } },
                                        { "sender.supreme_strife.team_id": { "$in": [Bson::Null, Bson::Int32(0), Bson::Int64(0)] } },
                                    ]
                                },
                            ]
                        })
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_time": -1,
                    "sender.supreme_strife.battle_id": 1,
                })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! {
                            "sender.supreme_strife.battle_id": { "$gt": "" },
                            "sender.supreme_strife.team_id": { "$gt": 0 },
                        })
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.kvk": 1,
                    "sender.server_season": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
            // IndexModel::builder()
            //     .keys(doc! {
            //         "metadata.mail_role": 1,
            //         "sender.session": 1,
            //         "metadata.mail_time": -1,
            //     })
            //     .build(),
            // IndexModel::builder()
            //     .keys(doc! { "sender.rally": 1, "metadata.mail_time": -1 })
            //     .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1, "sender.rally": 1 })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! {
                            "sender.rally": true,
                            "opponents.player_id": { "$gt": 0 },
                        })
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! { "opponents.rally": 1, "metadata.mail_time": -1 })
                .build(),
            // IndexModel::builder()
            //     .keys(doc! { "sender.alliance_building_id": 1, "metadata.mail_time": -1 })
            //     .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1, "sender.alliance_building_id": 1 })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! {
                            "sender.alliance_building_id": { "$gt": 0 },
                            "opponents.player_id": { "$gt": 0 },
                        })
                        .build(),
                )
                .build(),
            // IndexModel::builder()
            //     .keys(doc! { "opponents.alliance_building_id": 1, "metadata.mail_time": -1 })
            //     .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_time": -1, "opponents.alliance_building_id": 1 })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! {
                            "opponents.alliance_building_id": { "$gt": 0 },
                            "opponents.player_id": { "$gt": 0 },
                        })
                        .build(),
                )
                .build(),
        ];

        for model in models {
            self.mails_battle.create_index(model).await?;
        }

        let duel_models = vec![
            IndexModel::builder()
                .keys(doc! { "sender.duel.team_id": 1, "metadata.mail_time": 1 })
                .build(),
        ];

        for model in duel_models {
            self.mails_duelbattle2.create_index(model).await?;
        }

        let ark_battle_results_models = vec![
            IndexModel::builder()
                .keys(
                    doc! { "metadata.mail_receiver": 1, "body.type": 1, "metadata.mail_time": -1 },
                )
                .build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_receiver": 1, "metadata.mail_id": 1 })
                .build(),
        ];

        for model in ark_battle_results_models {
            self.mails_alliance_aoobattleresults.create_index(model).await?;
        }

        let ark_secondary_models = vec![
            IndexModel::builder()
                .keys(doc! { "metadata.mail_receiver": 1, "metadata.mail_time": -1 })
                .build(),
        ];

        for model in ark_secondary_models.clone() {
            self.mails_alliance_aoobattleinfo.create_index(model).await?;
        }
        for model in ark_secondary_models.clone() {
            self.mails_alliance_aooindividualresults.create_index(model).await?;
        }
        for model in ark_secondary_models {
            self.mails_alliance_aooregistration.create_index(model).await?;
        }

        let barbarian_fort_models = vec![
            IndexModel::builder().keys(doc! { "body.sub_param": 1, "body.sub_type": 1 }).build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_receiver": 1,
                    "body.sub_type": 1,
                    "body.sub_param": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_receiver": 1,
                    "body.sub_type": 1,
                    "body.sub_param": 1,
                    "body.content.level": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
        ];

        for model in barbarian_fort_models {
            self.mails_system_barbarianfort.create_index(model).await?;
        }

        let kahar_treasure_models = vec![
            IndexModel::builder().keys(doc! { "metadata.mail_time": -1 }).build(),
            IndexModel::builder()
                .keys(doc! { "metadata.mail_receiver": 1, "metadata.mail_time": -1 })
                .build(),
        ];

        for model in kahar_treasure_models {
            self.mails_system_kahartreasure.create_index(model).await?;
        }

        let baulur_models = vec![
            IndexModel::builder().keys(doc! { "npc.type": 1 }).build(),
            IndexModel::builder()
                .keys(doc! {
                    "metadata.mail_receiver": 1,
                    "participants.player_id": 1,
                    "npc.type": 1,
                    "metadata.mail_time": -1,
                })
                .build(),
        ];

        for model in baulur_models {
            self.mails_barcanyonkillboss.create_index(model).await?;
        }

        self.mails_eventmemberlootreport
            .create_index(
                IndexModel::builder()
                    .keys(doc! {
                        "metadata.mail_receiver": 1,
                        "participants.player_id": 1,
                        "boss.id": 1,
                        "metadata.mail_time": -1,
                    })
                    .build(),
            )
            .await?;
        self.mails_eventmemberlootreport
            .create_index(IndexModel::builder().keys(doc! { "boss.id": 1 }).build())
            .await?;

        let rss_models = vec![
            IndexModel::builder()
                .keys(doc! { "metadata.mail_receiver": 1, "metadata.mail_time": -1 })
                .build(),
        ];

        for model in rss_models {
            self.mails_rss.create_index(model).await?;
        }

        let claimed_governor_models = vec![
            IndexModel::builder()
                .keys(doc! { "governorId": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder().keys(doc! { "discordId": 1, "governorId": 1 }).build(),
            IndexModel::builder().keys(doc! { "discordId": 1, "createdAt": -1 }).build(),
            IndexModel::builder()
                .keys(doc! { "discordId": 1, "default": -1, "createdAt": -1 })
                .build(),
            IndexModel::builder()
                .keys(doc! { "discordId": 1, "default": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .partial_filter_expression(doc! { "default": true })
                        .build(),
                )
                .build(),
        ];

        for model in claimed_governor_models {
            self.claimed_governors.create_index(model).await?;
        }

        let precomputed_barbarian_models = vec![
            IndexModel::builder()
                .keys(doc! { "kind": 1, "level": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        ];

        for model in precomputed_barbarian_models {
            self.g_rok_prec_barbarian.create_index(model).await?;
        }

        let precomputed_barbarian_fort_models = vec![
            IndexModel::builder()
                .keys(doc! { "kind": 1, "level": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        ];

        for model in precomputed_barbarian_fort_models {
            self.g_rok_prec_barbarianfort.create_index(model).await?;
        }

        let precomputed_baulur_models = vec![
            IndexModel::builder()
                .keys(doc! { "kind": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        ];

        for model in precomputed_baulur_models {
            self.g_rok_prec_baulur.create_index(model).await?;
        }

        let precomputed_kahar_treasure_models = vec![
            IndexModel::builder()
                .keys(doc! { "kind": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        ];

        for model in precomputed_kahar_treasure_models {
            self.g_rok_prec_kahartreasure.create_index(model).await?;
        }

        self.g_rok_prec_karuakceremony
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "kind": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;

        self.g_rok_prec_drastc
            .create_index(
                IndexModel::builder()
                    .keys(doc! {
                        "primary_commander_id": 1,
                        "secondary_commander_id": 1,
                    })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;

        let precomputed_commander_pairing_v2_models = vec![
            IndexModel::builder()
                .keys(doc! { "k": 1, "p": 1, "s": 1, "g": -1, "m": 1, "q": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
            IndexModel::builder().keys(doc! { "g": 1 }).build(),
        ];

        for model in precomputed_commander_pairing_v2_models {
            self.g_rok_prec_cmdr_pairings_v2.create_index(model).await?;
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

    /// Access Ark of Osiris battle results mails.
    pub fn alliance_aoobattleresults_collection(&self) -> &Collection<Document> {
        &self.mails_alliance_aoobattleresults
    }

    /// Access Ark of Osiris battle info mails.
    pub fn alliance_aoobattleinfo_collection(&self) -> &Collection<Document> {
        &self.mails_alliance_aoobattleinfo
    }

    /// Access Ark of Osiris individual results mails.
    pub fn alliance_aooindividualresults_collection(&self) -> &Collection<Document> {
        &self.mails_alliance_aooindividualresults
    }

    /// Access Ark of Osiris registration mails.
    pub fn alliance_aooregistration_collection(&self) -> &Collection<Document> {
        &self.mails_alliance_aooregistration
    }

    /// Access the system barbarian fort mail collection.
    pub fn system_barbarian_fort_collection(&self) -> &Collection<Document> {
        &self.mails_system_barbarianfort
    }

    /// Access the system Kahar treasure mail collection.
    pub fn system_kahar_treasure_collection(&self) -> &Collection<Document> {
        &self.mails_system_kahartreasure
    }

    /// Access Karuak Ceremony member loot reports.
    pub fn event_member_loot_report_collection(&self) -> &Collection<Document> {
        &self.mails_eventmemberlootreport
    }

    /// Access the bar canyon kill boss mail collection.
    pub fn barcanyonkillboss_collection(&self) -> &Collection<Document> {
        &self.mails_barcanyonkillboss
    }

    /// Access the RSS mail collection.
    pub fn rss_collection(&self) -> &Collection<Document> {
        &self.mails_rss
    }

    /// Access claimed governor binds.
    pub fn claimed_governors_collection(&self) -> &Collection<Document> {
        &self.claimed_governors
    }

    /// Access precomputed barbarian aggregates.
    pub fn precomputed_barbarian_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_barbarian
    }

    /// Access precomputed barbarian fort aggregates.
    pub fn precomputed_barbarian_fort_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_barbarianfort
    }

    /// Access precomputed Baulur aggregates.
    pub fn precomputed_baulur_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_baulur
    }

    /// Access precomputed Kahar treasure aggregates.
    pub fn precomputed_kahar_treasure_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_kahartreasure
    }

    /// Access precomputed Karuak Ceremony boss aggregates.
    pub fn precomputed_karuak_ceremony_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_karuakceremony
    }

    /// Access materialized DRASTC scores for Combat Lab.
    pub fn precomputed_drastc_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_drastc
    }

    /// Access compact, chunked Combat Lab commander pairing aggregates.
    pub fn precomputed_commander_pairings_v2_collection(&self) -> &Collection<Document> {
        &self.g_rok_prec_cmdr_pairings_v2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_visible_battle_reports_exclude_only_the_test_client() {
        let filter = exclude_test_client_filter();
        assert_eq!(filter, doc! { "sender.app_id": { "$ne": 10_088_010_i64 } });
        assert!(!filter_matches_sender_app_id(&filter, TEST_CLIENT_APP_ID));
        assert!(filter_matches_sender_app_id(&filter, 2_104_267));
    }

    fn filter_matches_sender_app_id(filter: &Document, app_id: i64) -> bool {
        filter
            .get_document("sender.app_id")
            .and_then(|condition| condition.get_i64("$ne"))
            .is_ok_and(|excluded_app_id| app_id != excluded_app_id)
    }
}
