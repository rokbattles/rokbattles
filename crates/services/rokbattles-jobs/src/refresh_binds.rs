//! Keep claimed governor names and avatars in sync with recent battle data.

use core_bson::{bson_integer_to_i64, nested_string, nested_value};
use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Document, doc},
};
use rokbattles_api::db::ReportsStore;

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

/// Update claimed governor binds from the newest sender snapshot available.
pub async fn refresh_claimed_governor_bindings(
    reports_store: &ReportsStore,
) -> Result<RefreshBindsStats, JobsError> {
    refresh_claimed_governor_bindings_from_collections(
        reports_store.claimed_governors_collection(),
        reports_store.battle_collection(),
    )
    .await
}

async fn refresh_claimed_governor_bindings_from_collections(
    claimed_governors: &Collection<Document>,
    battle_reports: &Collection<Document>,
) -> Result<RefreshBindsStats, JobsError> {
    let distinct_ids = claimed_governors.distinct("governorId", doc! {}).await?;
    let governor_ids = distinct_ids.iter().filter_map(bson_integer_to_i64).collect::<Vec<_>>();

    let mut stats =
        RefreshBindsStats { governors_seen: governor_ids.len(), ..RefreshBindsStats::default() };

    if governor_ids.is_empty() {
        return Ok(stats);
    }

    let snapshots = fetch_latest_sender_snapshots(battle_reports, &governor_ids).await?;

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

async fn fetch_latest_sender_snapshots(
    battle_reports: &Collection<Document>,
    governor_ids: &[i64],
) -> Result<Vec<LatestGovernorSnapshot>, JobsError> {
    let pipeline = vec![
        doc! {
            "$match": {
                "sender.player_id": { "$in": governor_ids }
            }
        },
        doc! {
            "$sort": {
                "sender.player_id": 1,
                "metadata.mail_time": -1,
            }
        },
        doc! {
            "$group": {
                "_id": "$sender.player_id",
                "governorName": { "$first": "$sender.player_name" },
                "governorAvatar": { "$first": "$sender.avatar_url" },
            }
        },
    ];

    let mut cursor = battle_reports.aggregate(pipeline).await?;

    let mut snapshots = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next?;
        if let Some(snapshot) = map_latest_snapshot_document(&document) {
            snapshots.push(snapshot);
        }
    }

    Ok(snapshots)
}

fn map_latest_snapshot_document(document: &Document) -> Option<LatestGovernorSnapshot> {
    Some(LatestGovernorSnapshot {
        governor_id: nested_i64_exact(document, &["_id"])?,
        governor_name: nested_string(document, &["governorName"]),
        governor_avatar: nested_string(document, &["governorAvatar"]),
    })
}

fn nested_i64_exact(document: &Document, path: &[&str]) -> Option<i64> {
    nested_value(document, path).and_then(bson_integer_to_i64)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::{bson_integer_to_i64, map_latest_snapshot_document};

    #[test]
    fn bson_integer_to_i64_accepts_stored_integer_id_types() {
        assert_eq!(bson_integer_to_i64(&Bson::Int32(1001)), Some(1001));
        assert_eq!(bson_integer_to_i64(&Bson::Int64(1001)), Some(1001));
    }

    #[test]
    fn bson_integer_to_i64_rejects_float_ids() {
        assert_eq!(bson_integer_to_i64(&Bson::Double(1001.0)), None);
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
