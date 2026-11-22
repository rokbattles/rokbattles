//! Keep claimed governor names and avatars in sync with recent battle data.

use futures::TryStreamExt;
use mongodb::bson::{Document, doc};
use rokbattles_api::db::{ReportsStore, exclude_test_reports_filter};
use rokbattles_bson::{bson_integer_to_i64, nested_string};

use crate::error::JobsError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LatestGovernorSnapshot {
    governor_id: i64,
    governor_name: Option<String>,
    governor_avatar: Option<String>,
}

/// Counts from one governor bind refresh run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefreshBindsStats {
    pub governors_seen: usize,
    pub governors_refreshed: usize,
    pub claims_matched: u64,
    pub claims_updated: u64,
}

/// Update claimed governor binds from the newest eligible sender snapshot.
pub async fn refresh_claimed_governor_bindings(
    reports_store: &ReportsStore,
) -> Result<RefreshBindsStats, JobsError> {
    let claimed_governors = reports_store.claimed_governors_collection();
    let pipeline = [
        // distinct("governorId") also expands legacy array values.
        doc! { "$unwind": "$governorId" },
        doc! { "$group": { "_id": "$governorId" } },
        doc! { "$match": { "$expr": { "$in": [{ "$type": "$_id" }, ["int", "long"]] } } },
        doc! {
            "$lookup": {
                "from": reports_store.battle_collection().name(),
                "localField": "_id",
                "foreignField": "sender.player_id",
                "pipeline": [
                    { "$match": exclude_test_reports_filter() },
                    // Array IDs can match an equality join but form a separate group.
                    { "$match": { "$expr": { "$ne": [{ "$type": "$sender.player_id" }, "array"] } } },
                    { "$sort": { "metadata.mail_time": -1 } },
                    { "$limit": 1 },
                    { "$project": {
                        "_id": "$sender.player_id",
                        "governorName": "$sender.player_name",
                        "governorAvatar": "$sender.avatar_url",
                    } },
                ],
                "as": "snapshot",
            }
        },
        doc! { "$unwind": { "path": "$snapshot", "preserveNullAndEmptyArrays": true } },
    ];

    let mut cursor = claimed_governors.aggregate(pipeline).await?;
    let mut stats = RefreshBindsStats::default();
    let mut snapshots = Vec::new();
    while let Some(row) = cursor.try_next().await? {
        stats.governors_seen += 1;
        if let Some(snapshot) =
            row.get_document("snapshot").ok().and_then(map_latest_snapshot_document)
        {
            snapshots.push(snapshot);
        }
    }

    // Finish reading before updating claims, so a failed lookup can't leave a partial refresh.
    for snapshot in snapshots {
        let update_result = claimed_governors
            .update_many(
                doc! { "governorId": snapshot.governor_id },
                doc! {
                    "$set": {
                        "governorName": snapshot.governor_name,
                        "governorAvatar": snapshot.governor_avatar,
                    }
                },
            )
            .await?;

        if update_result.matched_count > 0 {
            stats.governors_refreshed += 1;
            stats.claims_matched += update_result.matched_count;
            stats.claims_updated += update_result.modified_count;
        }
    }

    Ok(stats)
}

fn map_latest_snapshot_document(document: &Document) -> Option<LatestGovernorSnapshot> {
    Some(LatestGovernorSnapshot {
        governor_id: document.get("_id").and_then(bson_integer_to_i64)?,
        governor_name: nested_string(document, &["governorName"]),
        governor_avatar: nested_string(document, &["governorAvatar"]),
    })
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::map_latest_snapshot_document;

    #[test]
    fn map_latest_snapshot_document_skips_non_integer_ids_matched_by_lookup() {
        for id in [Bson::Double(1001.0), Bson::Array(vec![Bson::Int32(1001)])] {
            assert!(map_latest_snapshot_document(&doc! { "_id": id }).is_none());
        }
    }

    #[test]
    fn map_latest_snapshot_document_maps_fields() {
        let document = doc! {
            "_id": 1001_i64,
            "governorName": "Alpha",
            "governorAvatar": "avatar.png",
        };

        let snapshot = map_latest_snapshot_document(&document).expect("snapshot");
        assert_eq!(snapshot.governor_id, 1001);
        assert_eq!(snapshot.governor_name.as_deref(), Some("Alpha"));
        assert_eq!(snapshot.governor_avatar.as_deref(), Some("avatar.png"));
    }

    #[test]
    fn map_latest_snapshot_document_skips_missing_governor_id() {
        let document = doc! {
            "governorName": "Alpha",
        };

        assert!(map_latest_snapshot_document(&document).is_none());
    }

    #[test]
    fn map_latest_snapshot_document_keeps_nullable_fields() {
        let document = doc! {
            "_id": 1001_i64,
            "governorName": Bson::Null,
            "governorAvatar": Bson::Null,
        };

        let snapshot = map_latest_snapshot_document(&document).expect("snapshot");
        assert_eq!(snapshot.governor_name, None);
        assert_eq!(snapshot.governor_avatar, None);
    }
}
