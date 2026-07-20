use std::collections::BTreeMap;

use core_bson::{bson_to_f64, bson_to_i64};
use drastc::DrastcConfidence;
use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Document, doc},
};

use super::{
    model::{PairingKey, Strategy},
    pipeline::build_supported_pairing_entries_pipeline,
};
use crate::error::JobsError;

pub(super) async fn read_pairing_confidences(
    source: &Collection<Document>,
    supported_pairings: &[PairingKey],
) -> Result<BTreeMap<PairingKey, DrastcConfidence>, JobsError> {
    let pipeline = build_confidence_pipeline(supported_pairings);
    let mut cursor = source.aggregate(pipeline).allow_disk_use(true).await?;
    let mut confidences = BTreeMap::new();

    while let Some(next) = cursor.next().await {
        if let Some((key, confidence)) = map_confidence_document(&next?) {
            confidences.insert(key, confidence);
        }
    }

    Ok(confidences)
}

pub(super) fn build_confidence_pipeline(supported_pairings: &[PairingKey]) -> Vec<Document> {
    let mut pipeline = build_supported_pairing_entries_pipeline(supported_pairings);
    pipeline.extend([
        doc! {
            "$match": {
                "strategy": Strategy::OpenField.as_str(),
            }
        },
        doc! {
            "$group": {
                "_id": {
                    "primary_commander_id": "$primary_commander_id",
                    "secondary_commander_id": "$secondary_commander_id",
                    "player_id": "$player_id",
                },
                "governor_battles": { "$sum": 1_i64 },
            }
        },
        doc! {
            "$group": {
                "_id": {
                    "primary_commander_id": "$_id.primary_commander_id",
                    "secondary_commander_id": "$_id.secondary_commander_id",
                },
                "total_battles": { "$sum": "$governor_battles" },
                "unique_governors": {
                    "$sum": {
                        "$cond": [{ "$gt": ["$_id.player_id", 0_i64] }, 1_i64, 0_i64]
                    }
                },
                "governor_battles_squared_sum": {
                    "$sum": { "$multiply": ["$governor_battles", "$governor_battles"] }
                },
            }
        },
        doc! {
            "$project": {
                "_id": 0,
                "primary_commander_id": "$_id.primary_commander_id",
                "secondary_commander_id": "$_id.secondary_commander_id",
                "total_battles": 1,
                "unique_governors": 1,
                "governor_battles_squared_sum": 1,
            }
        },
    ]);
    pipeline
}

fn map_confidence_document(document: &Document) -> Option<(PairingKey, DrastcConfidence)> {
    let key = PairingKey {
        primary_commander_id: direct_i64(document, "primary_commander_id")?,
        secondary_commander_id: direct_i64(document, "secondary_commander_id")?,
    };
    let total_battles = direct_i64(document, "total_battles")?;
    let unique_governors = direct_i64(document, "unique_governors")?;
    let confidence = DrastcConfidence::from_governor_distribution(
        u64::try_from(total_battles).unwrap_or_default(),
        u64::try_from(unique_governors).unwrap_or_default(),
        direct_f64(document, "governor_battles_squared_sum")?,
    );

    Some((key, confidence))
}

fn direct_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn direct_f64(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_pipeline_groups_open_field_battles_by_governor_then_pairing() {
        let pairing = PairingKey { primary_commander_id: 595, secondary_commander_id: 596 };
        let pipeline = build_confidence_pipeline(&[pairing]);
        let group_count = pipeline.iter().filter(|stage| stage.contains_key("$group")).count();
        let pairing_match = pipeline
            .iter()
            .filter_map(|stage| stage.get_document("$match").ok())
            .find(|matcher| matcher.contains_key("strategy"))
            .expect("open-field pairing match");

        assert_eq!(group_count, 2);
        assert_eq!(pairing_match.get_str("strategy"), Ok("open_field"));
        let leading_match = pipeline[0].get_document("$match").expect("leading match");
        assert_eq!(leading_match.get_array("$or").map(Vec::len), Ok(2));
        assert!(!format!("{pipeline:?}").contains("kill_points_gained"));
    }

    #[test]
    fn confidence_document_maps_aggregation_values() {
        let (key, confidence) = map_confidence_document(&doc! {
            "primary_commander_id": 595_i64,
            "secondary_commander_id": 596_i64,
            "total_battles": 111_512_i64,
            "unique_governors": 816_i64,
            "governor_battles_squared_sum": 437_627_902_i64,
        })
        .expect("confidence document");

        assert_eq!(key, PairingKey { primary_commander_id: 595, secondary_commander_id: 596 });
        assert_eq!(confidence.unique_governors, 816);
        assert!((confidence.effective_governors - 28.41).abs() < 0.005);
    }
}
