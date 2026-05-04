use std::sync::Arc;

use mongodb::{
    bson::{Document, doc},
    options::{FindOneOptions, FindOptions},
};

use crate::{
    error::ApiError, routes::governor::store_utils::fetch_collection_documents, state::AppState,
};

pub(crate) async fn fetch_ark_battle_results_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    limit: i64,
) -> Result<Vec<Document>, ApiError> {
    let options = FindOptions::builder()
        .sort(doc! { "metadata.mail_time": -1 })
        .limit(limit)
        .projection(doc! {
            "_id": 0,
            "metadata.mail_id": 1,
            "metadata.mail_time": 1,
            "body.win": 1,
            "body.alliance.id": 1,
            "alliances.alliance.id": 1,
            "alliances.alliance.name": 1,
            "alliances.alliance.abbreviation": 1,
            "alliances.score": 1,
            "alliances.members": 1,
            "alliances.members_max": 1,
            "alliances.is_blue": 1,
        })
        .build();

    let filter = ark_battle_results_history_filter(mail_receiver);
    fetch_collection_documents(
        state.reports_store.alliance_aoobattleresults_collection(),
        filter,
        options,
    )
    .await
}

pub(crate) async fn fetch_ark_battle_results_mail_by_id(
    state: &Arc<AppState>,
    mail_receiver: &str,
    mail_id: &str,
) -> Result<Option<Document>, ApiError> {
    let options = FindOneOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_id": 1,
            "metadata.mail_time": 1,
            "body.win": 1,
            "body.alliance.id": 1,
            "alliances.alliance.id": 1,
            "alliances.alliance.name": 1,
            "alliances.alliance.abbreviation": 1,
            "alliances.score": 1,
            "alliances.members": 1,
            "alliances.members_max": 1,
            "alliances.is_blue": 1,
        })
        .build();

    state
        .reports_store
        .alliance_aoobattleresults_collection()
        .find_one(doc! {
            "metadata.mail_receiver": mail_receiver,
            "metadata.mail_id": mail_id,
        })
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

pub(crate) async fn fetch_ark_battle_info_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<Document>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_id": 1,
            "metadata.mail_time": 1,
            "body.win": 1,
            "body.fights.team": 1,
            "body.fights.time": 1,
            "body.fights.win": 1,
        })
        .build();

    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(
        state.reports_store.alliance_aoobattleinfo_collection(),
        filter,
        options,
    )
    .await
}

pub(crate) async fn fetch_ark_individual_results_mails(
    state: &Arc<AppState>,
    mail_receiver: &str,
    time_match: &Document,
) -> Result<Vec<Document>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "metadata.mail_id": 1,
            "metadata.mail_time": 1,
            "body.team": 1,
            "body.win": 1,
            "overview.player_id": 1,
            "overview.player_name": 1,
            "overview.rank": 1,
            "overview.score": 1,
            "overview.total_results.battles": 1,
            "overview.total_results.kill_points": 1,
            "overview.total_results.severely_wounded": 1,
            "results.total_score": 1,
            "results.win_rate": 1,
            "results.battles_win": 1,
            "results.battles_lose": 1,
            "results.severely_wounded": 1,
            "results.kills": 1,
            "results.kill_score": 1,
            "results.flag_score": 1,
            "results.building_score": 1,
            "results.gather_score": 1,
            "results.units_healed": 1,
            "results.speedups": 1,
            "results.teleports": 1,
            "results.structures": 1,
            "pairings.primary_commander.id": 1,
            "pairings.secondary_commander.id": 1,
            "pairings.battles": 1,
            "pairings.battles_win": 1,
            "pairings.kill_count": 1,
            "pairings.kill_points": 1,
            "pairings.severely_wounded": 1,
        })
        .build();

    let filter = doc! {
        "$and": [
            { "metadata.mail_receiver": mail_receiver },
            time_match.clone(),
        ]
    };

    fetch_collection_documents(
        state.reports_store.alliance_aooindividualresults_collection(),
        filter,
        options,
    )
    .await
}

fn ark_battle_results_history_filter(mail_receiver: &str) -> Document {
    doc! {
        "metadata.mail_receiver": mail_receiver,
        "body.type": { "$ne": 14 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ark_battle_results_history_filter_excludes_type_14_matches() {
        let filter = ark_battle_results_history_filter("player_42");
        assert_eq!(filter.get_str("metadata.mail_receiver").ok(), Some("player_42"));
        let type_filter = filter.get_document("body.type").expect("body.type filter");
        assert_eq!(type_filter.get_i32("$ne").ok(), Some(14));
    }
}
