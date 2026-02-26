use mongodb::Collection;
use mongodb::bson::{Document, doc};

use crate::error::ApiError;
use crate::governor_bindings::snapshot::find_latest_sender_snapshot;

use super::mapper::bson_to_i64;
use super::types::RefreshBindsStats;

/// Update all claimed governor binds with the latest sender snapshot we can find.
pub(super) async fn refresh_claimed_governor_bindings(
    claimed_governors: &Collection<Document>,
    battle_reports: &Collection<Document>,
) -> Result<RefreshBindsStats, ApiError> {
    let distinct_ids = claimed_governors
        .distinct("governorId", doc! {})
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let governor_ids = distinct_ids
        .iter()
        .filter_map(bson_to_i64)
        .collect::<Vec<_>>();

    let mut stats = RefreshBindsStats {
        governors_seen: governor_ids.len(),
        ..RefreshBindsStats::default()
    };

    for governor_id in &governor_ids {
        let Some(snapshot) = find_latest_sender_snapshot(battle_reports, *governor_id).await?
        else {
            continue;
        };

        let update_result = claimed_governors
            .update_many(
                doc! { "governorId": *governor_id },
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
