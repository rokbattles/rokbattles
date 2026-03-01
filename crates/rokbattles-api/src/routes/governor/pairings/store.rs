use std::sync::Arc;

use mongodb::bson::{Document, doc};
use mongodb::options::FindOptions;

use crate::error::ApiError;
use crate::routes::governor::date_range::GovernorDateRange;
use crate::routes::governor::store_utils::fetch_collection_documents;
use crate::state::AppState;
use crate::time_utils::build_mail_time_match;

pub(crate) async fn fetch_pairings_mails(
    state: &Arc<AppState>,
    governor_id: i64,
    range: &GovernorDateRange,
    primary_commander_id: Option<i64>,
) -> Result<Vec<Document>, ApiError> {
    let mut and_filters = vec![
        doc! { "sender.player_id": governor_id },
        doc! { "opponents": { "$elemMatch": { "player_id": { "$nin": [-2, 0] } } } },
        build_mail_time_match(range.start_millis, range.end_millis),
    ];

    if let Some(primary_commander_id) = primary_commander_id {
        and_filters.push(doc! { "sender.commanders.primary.id": primary_commander_id });
    }

    let filter = doc! { "$and": and_filters };
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_time": 1,
            "timeline.start_timestamp": 1,
            "sender.commanders.primary.id": 1,
            "sender.commanders.secondary.id": 1,
            "sender.commanders.primary.equipment": 1,
            "sender.commanders.primary.formation": 1,
            "sender.commanders.primary.armaments": 1,
            "sender.commanders.secondary.armaments": 1,
            "opponents.player_id": 1,
            "opponents.start_tick": 1,
            "opponents.end_tick": 1,
            "opponents.commanders.primary.id": 1,
            "opponents.commanders.secondary.id": 1,
            "opponents.battle_results.sender.kill_points": 1,
            "opponents.battle_results.sender.dead": 1,
            "opponents.battle_results.sender.severely_wounded": 1,
            "opponents.battle_results.sender.slightly_wounded": 1,
            "opponents.battle_results.opponent.kill_points": 1,
            "opponents.battle_results.opponent.dead": 1,
            "opponents.battle_results.opponent.severely_wounded": 1,
            "opponents.battle_results.opponent.slightly_wounded": 1,
        })
        .build();

    fetch_collection_documents(state.reports_store.battle_collection(), filter, options).await
}
