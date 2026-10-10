use futures::TryStreamExt;
use mongodb::{
    bson::{Document, doc},
    options::FindOptions,
};
use rustc_hash::FxHashSet;

use super::{BATTLE_LIFETIME_MS, QUERY_TIMEOUT, aggregate::March};
use crate::{
    error::ApiError, routes::reports::battle::list_mapper::build_battle_list_projection,
    state::AppState,
};

fn report_filter(governor: i64, start: i64, now: i64) -> Document {
    doc! {
        "sender.player_id": governor,
        "opponents.player_id": {"$gt": 2},
        "metadata.mail_time": {
            "$gte": start.saturating_mul(1000),
            "$lte": now.saturating_mul(1000)
        }
    }
}

pub(super) async fn fetch_reports(
    state: &AppState,
    governor: i64,
    limit: usize,
    now: i64,
) -> Result<Vec<Document>, ApiError> {
    let recent_start = now - BATTLE_LIFETIME_MS;
    let options = FindOptions::builder()
        .sort(doc! {"metadata.mail_time": -1})
        .projection(doc! {
            "metadata.mail_id": 1,
            "metadata.server_id": 1,
            "sender.player_id": 1,
            "sender.tracking_key": 1
        })
        .batch_size(100)
        .max_time(QUERY_TIMEOUT)
        .build();

    let mut cursor = state
        .reports_store
        .battle_collection()
        .find(report_filter(governor, recent_start, now))
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut marches = FxHashSet::default();

    while let Some(report) =
        cursor.try_next().await.map_err(|error| ApiError::internal(error.to_string()))?
    {
        if let Some(march) = March::from_report(&report) {
            marches.insert(march);
        }
        if marches.len() >= limit {
            break;
        }
    }

    if marches.is_empty() {
        return Ok(Vec::new());
    }

    // Discover recent marches first, then include their older child reports too.
    // Applying the five-minute cutoff to the second query would lose march totals.
    let mut filter = report_filter(governor, 0, now);
    filter.insert("$or", marches.iter().map(March::filter).collect::<Vec<_>>());

    state
        .reports_store
        .battle_collection()
        .find(filter)
        .sort(doc! {"metadata.mail_time": -1})
        .projection(build_battle_list_projection())
        .max_time(QUERY_TIMEOUT)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .try_collect()
        .await
        .map_err(|error: mongodb::error::Error| ApiError::internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_only_by_governor_player_opponents_and_time() {
        assert_eq!(
            report_filter(10, 1000, 2000),
            doc! {
                "sender.player_id": 10_i64,
                "opponents.player_id": {"$gt": 2},
                "metadata.mail_time": {"$gte": 1_000_000_i64, "$lte": 2_000_000_i64}
            }
        );
    }
}
