//! Shared inputs and storage isolation for Combat Lab precomputation.

use mongodb::{
    Collection, IndexModel,
    bson::{Document, Regex, doc},
    options::{Hint, IndexOptions},
};
use rokbattles_api::db::ReportsStore;
use rokbattles_drastc::{PRESOC_RAGE_TABLE, RageTable, SOC_RAGE_TABLE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatLabSeason {
    Soc,
    PreSoc,
}

impl CombatLabSeason {
    pub(crate) fn rage_table(self) -> RageTable {
        match self {
            Self::Soc => SOC_RAGE_TABLE,
            Self::PreSoc => PRESOC_RAGE_TABLE,
        }
    }

    pub(crate) fn scope_pipeline(self, mut pipeline: Vec<Document>) -> Vec<Document> {
        if self == Self::PreSoc {
            // Separate exact seasons and literal prefixes so the season index gets narrow bounds.
            // This preserves the report API's sender-based season semantics.
            // MongoDB coalesces this with the existing leading match before reading reports.
            pipeline.insert(
                0,
                doc! {
                    "$match": { "sender.server_season": { "$in": [
                        "1", "2",
                        Regex { pattern: r"^1\..*$".into(), options: String::new() },
                        Regex { pattern: r"^2\..*$".into(), options: String::new() },
                    ] } }
                },
            );
        }
        pipeline
    }

    pub(crate) fn source_hint(self) -> Hint {
        Hint::Keys(match self {
            Self::Soc => {
                doc! { "metadata.mail_time": -1, "metadata.kvk": 1, "opponents.player_id": 1 }
            }
            Self::PreSoc => {
                doc! { "metadata.kvk": 1, "sender.server_season": 1, "metadata.mail_time": -1 }
            }
        })
    }

    pub(crate) fn drastc_collection(self, store: &ReportsStore) -> Collection<Document> {
        match self {
            Self::Soc => store.precomputed_drastc_collection().clone(),
            Self::PreSoc => presoc_collection(store, "g_rok_prec_drastc_presoc"),
        }
    }

    pub(crate) fn pairings_collection(self, store: &ReportsStore) -> Collection<Document> {
        match self {
            Self::Soc => store.precomputed_commander_pairings_v2_collection().clone(),
            Self::PreSoc => presoc_collection(store, "g_rok_prec_cmdr_pairings_v2_presoc"),
        }
    }

    pub(crate) async fn ensure_drastc_index(
        self,
        output: &Collection<Document>,
    ) -> mongodb::error::Result<()> {
        if self == Self::PreSoc {
            output
                .create_index(
                    IndexModel::builder()
                        .keys(doc! { "primary_commander_id": 1, "secondary_commander_id": 1 })
                        .options(IndexOptions::builder().unique(true).build())
                        .build(),
                )
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_pairings_indexes(
        self,
        output: &Collection<Document>,
    ) -> mongodb::error::Result<()> {
        if self == Self::PreSoc {
            output
                .create_indexes([
                    IndexModel::builder()
                        .keys(doc! { "k": 1, "p": 1, "s": 1, "g": -1, "m": 1, "q": 1 })
                        .options(IndexOptions::builder().unique(true).build())
                        .build(),
                    IndexModel::builder().keys(doc! { "g": 1 }).build(),
                ])
                .await?;
        }
        Ok(())
    }
}

fn presoc_collection(store: &ReportsStore, name: &str) -> Collection<Document> {
    let source = store.battle_collection();
    source.client().database(&source.namespace().db).collection(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presoc_matches_only_seasons_one_and_two_with_optional_dot_suffixes() {
        let pipeline = CombatLabSeason::PreSoc.scope_pipeline(vec![]);
        let values = pipeline[0]
            .get_document("$match")
            .expect("match")
            .get_document("sender.server_season")
            .expect("sender season")
            .get_array("$in")
            .expect("season alternatives");
        let matches = |value: &str| {
            values.iter().any(|candidate| match candidate {
                mongodb::bson::Bson::String(exact) => exact == value,
                mongodb::bson::Bson::RegularExpression(regex) => {
                    regex::Regex::new(&regex.pattern).expect("valid regex").is_match(value)
                }
                _ => false,
            })
        };
        for value in ["1", "1.1", "2", "2.1", "1.2", "2.10"] {
            assert!(matches(value), "excluded {value}");
        }
        for value in
            ["", "-1", "0", "3", "3.1", "4", "4.1", "99", "100", "100.1", "10", "21", "1x1", " 1"]
        {
            assert!(!matches(value), "included {value}");
        }
    }

    #[test]
    fn soc_preserves_the_existing_report_scope() {
        let pipeline = vec![doc! { "$match": { "metadata.kvk": true } }];
        assert_eq!(CombatLabSeason::Soc.scope_pipeline(pipeline.clone()), pipeline);
    }

    #[test]
    fn rage_tables_are_selected_by_season() {
        assert_eq!(CombatLabSeason::Soc.rage_table(), SOC_RAGE_TABLE);
        assert_eq!(CombatLabSeason::PreSoc.rage_table(), PRESOC_RAGE_TABLE);
    }
}
