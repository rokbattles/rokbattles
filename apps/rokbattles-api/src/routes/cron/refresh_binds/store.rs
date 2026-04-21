use futures::StreamExt;
use mongodb::{
    Collection,
    bson::{Document, doc},
};

use super::types::RefreshBindsStats;
use crate::{
    bson_utils::{bson_to_i64_exact, nested_i64_exact, nested_string},
    error::ApiError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct LatestGovernorSnapshot {
    governor_id: i64,
    governor_name: Option<String>,
    governor_avatar: Option<String>,
}

/// Update claimed governor binds using the latest sender snapshot we can find.
pub(super) async fn refresh_claimed_governor_bindings(
    claimed_governors: &Collection<Document>,
    battle_reports: &Collection<Document>,
) -> Result<RefreshBindsStats, ApiError> {
    let distinct_ids = claimed_governors
        .distinct("governorId", doc! {})
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let governor_ids = distinct_ids.iter().filter_map(bson_to_i64_exact).collect::<Vec<_>>();

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
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;

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
) -> Result<Vec<LatestGovernorSnapshot>, ApiError> {
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

    let mut cursor = battle_reports
        .aggregate(pipeline)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut snapshots = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
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

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, doc};

    use super::map_latest_snapshot_document;

    #[test]
    fn maps_latest_snapshot_document() {
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
    fn skips_snapshot_document_without_governor_id() {
        let document = doc! {
            "governorName": "Alpha",
        };

        assert!(map_latest_snapshot_document(&document).is_none());
    }

    #[test]
    fn keeps_nullable_snapshot_fields() {
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
