use std::sync::Arc;

use futures::TryStreamExt;
use mongodb::{
    bson::{Bson, Document, doc, from_document},
    options::FindOptions,
};

use super::types::{PairingAggregateResponse, PairingTotals};
use crate::{
    db::exclude_test_client_filter,
    error::ApiError,
    routes::governor::{
        date_range::GovernorDateRange,
        pairings::query::{
            PairingsActivity, PairingsBattleType, build_excluded_activity_conditions,
            build_excluded_battle_type_conditions,
        },
        store_utils::fetch_collection_documents,
    },
    state::AppState,
    time_utils::build_mail_time_match,
};

pub(crate) async fn fetch_pairings_overview(
    state: &Arc<AppState>,
    governor_id: i64,
    range: &GovernorDateRange,
    exclude_activities: &[PairingsActivity],
    exclude_battles: &[PairingsBattleType],
) -> Result<Vec<PairingAggregateResponse>, ApiError> {
    let filter =
        build_pairings_mail_filter(governor_id, range, None, exclude_activities, exclude_battles);

    let mut cursor = state
        .reports_store
        .battle_collection()
        .aggregate(overview_pipeline(filter))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut items = Vec::new();
    while let Some(doc) =
        cursor.try_next().await.map_err(|error| ApiError::internal(error.to_string()))?
    {
        items.push(map_overview_document(doc)?);
    }

    items.sort_by(|a, b| {
        b.totals.kill_score.cmp(&a.totals.kill_score).then_with(|| b.count.cmp(&a.count))
    });

    Ok(items)
}

fn map_overview_document(doc: Document) -> Result<PairingAggregateResponse, ApiError> {
    let ids = doc.get_document("_id").map_err(|error| ApiError::internal(error.to_string()))?;
    let primary_commander_id = ids.get_i64("primary").unwrap_or_default();
    let secondary_commander_id = ids.get_i64("secondary").unwrap_or_default();
    let count = doc.get_i64("count").unwrap_or_default();
    let positive_trades = doc.get_i64("positiveTrades").unwrap_or_default();

    let mut totals: PairingTotals =
        from_document(doc).map_err(|error| ApiError::internal(error.to_string()))?;
    totals.dps = totals.enemy_wounded + totals.enemy_severely_wounded;
    totals.sps = totals.enemy_severely_wounded;
    totals.tps = totals.severely_wounded;

    totals.weighted_trade_percent = if totals.kill_score == totals.enemy_kill_score {
        100.0
    } else if totals.enemy_kill_score <= 0 {
        0.0
    } else {
        totals.kill_score as f64 / totals.enemy_kill_score as f64 * 100.0
    };
    totals.hps = if totals.battle_duration > 0 {
        totals.healing_count as f64 / (totals.battle_duration as f64 / 1000.0)
    } else {
        0.0
    };

    Ok(PairingAggregateResponse {
        primary_commander_id,
        secondary_commander_id,
        count,
        positive_trades,
        totals,
    })
}

fn overview_pipeline(filter: Document) -> Vec<Document> {
    let mut group = doc! {
        "_id": {
            "primary": { "$convert": { "input": "$sender.commanders.primary.id", "to": "long", "onError": 0_i64, "onNull": 0_i64 } },
            "secondary": { "$convert": { "input": "$sender.commanders.secondary.id", "to": "long", "onError": 0_i64, "onNull": 0_i64 } },
        },
        "count": { "$sum": 1_i64 },
        "positiveTrades": { "$sum": { "$cond": [
            { "$gt": [
                { "$convert": { "input": "$opponents.battle_results.sender.kill_points", "to": "long", "onError": 0_i64, "onNull": 0_i64 } },
                { "$convert": { "input": "$opponents.battle_results.opponent.kill_points", "to": "long", "onError": 0_i64, "onNull": 0_i64 } },
            ] },
            1_i64,
            0_i64,
        ] } },
        "battleDuration": { "$sum": { "$max": [0_i64, { "$subtract": ["$endMillis", "$startMillis"] }] } },
    };

    for (target, side, field) in [
        ("killScore", "sender", "kill_points"),
        ("deaths", "sender", "dead"),
        ("severelyWounded", "sender", "severely_wounded"),
        ("wounded", "sender", "slightly_wounded"),
        ("healingCount", "sender", "heal"),
        ("enemyHealingCount", "opponent", "heal"),
        ("powerLoss", "sender", "power"),
        ("atkPowerLoss", "sender", "attack_power"),
        ("skillPowerLoss", "sender", "skill_power"),
        ("enemyKillScore", "opponent", "kill_points"),
        ("enemyDeaths", "opponent", "dead"),
        ("enemySeverelyWounded", "opponent", "severely_wounded"),
        ("enemyWounded", "opponent", "slightly_wounded"),
        ("enemyPowerLoss", "opponent", "power"),
        ("enemyAtkPowerLoss", "opponent", "attack_power"),
        ("enemySkillPowerLoss", "opponent", "skill_power"),
    ] {
        group.insert(target, doc! { "$sum": { "$convert": { "input": format!("$opponents.battle_results.{side}.{field}"), "to": "long", "onNull": 0_i64, "onError": 0_i64 } } });
    }

    vec![
        doc! { "$match": filter },
        doc! { "$project": {
            "sender.commanders.primary.id": 1,
            "sender.commanders.secondary.id": 1,
            "timeline.start_timestamp": 1,
            "opponents.player_id": 1,
            "opponents.start_tick": 1,
            "opponents.end_tick": 1,
            "opponents.battle_results": 1,
        } },
        doc! { "$unwind": "$opponents" },
        doc! { "$match": { "opponents.player_id": { "$gt": 0 }, "sender.commanders.primary.id": { "$gt": 0 } } },
        doc! { "$set": {
            "startMillis": normalized_millis("$opponents.start_tick", None),
            "endMillis": normalized_millis("$opponents.end_tick", Some("$opponents.start_tick")),
        } },
        doc! { "$group": group },
    ]
}

fn normalized_millis(tick: &str, fallback: Option<&str>) -> Document {
    let tick_value = fallback.map_or_else(
        || Bson::String(tick.to_string()),
        |fallback| Bson::Document(doc! { "$ifNull": [tick, fallback] }),
    );

    doc! { "$let": {
        "vars": { "value": { "$add": [
            { "$convert": { "input": "$timeline.start_timestamp", "to": "double", "onError": 0.0, "onNull": 0.0 } },
            { "$convert": { "input": tick_value, "to": "double", "onError": 0.0, "onNull": 0.0 } },
        ] } },
        "in": { "$toLong": { "$switch": {
            "branches": [
                { "case": { "$lt": [{ "$abs": "$$value" }, 1e12] }, "then": { "$multiply": ["$$value", 1000.0] } },
                { "case": { "$gte": [{ "$abs": "$$value" }, 1e17] }, "then": { "$divide": ["$$value", 1e6] } },
                { "case": { "$gte": [{ "$abs": "$$value" }, 1e14] }, "then": { "$divide": ["$$value", 1e3] } },
            ],
            "default": "$$value",
        } } },
    } }
}

pub(crate) async fn fetch_pairings_mails(
    state: &Arc<AppState>,
    governor_id: i64,
    range: &GovernorDateRange,
    pairing: Option<(i64, i64)>,
    exclude_activities: &[PairingsActivity],
    exclude_battles: &[PairingsBattleType],
) -> Result<Vec<Document>, ApiError> {
    let filter = build_pairings_mail_filter(
        governor_id,
        range,
        pairing,
        exclude_activities,
        exclude_battles,
    );
    let options = FindOptions::builder().projection(pairings_projection()).build();

    fetch_collection_documents(state.reports_store.battle_collection(), filter, options).await
}

fn build_pairings_mail_filter(
    governor_id: i64,
    range: &GovernorDateRange,
    pairing: Option<(i64, i64)>,
    exclude_activities: &[PairingsActivity],
    exclude_battles: &[PairingsBattleType],
) -> Document {
    let mut and_filters = vec![
        exclude_test_client_filter(),
        doc! { "sender.player_id": governor_id },
        doc! { "opponents.player_id": { "$gt": 0 } },
        build_mail_time_match(range.start_millis, range.end_millis),
    ];

    if let Some((primary_commander_id, secondary_commander_id)) = pairing {
        and_filters.push(doc! { "sender.commanders.primary.id": primary_commander_id });
        and_filters.push(if secondary_commander_id == 0 {
            doc! { "sender.commanders.secondary.id": { "$in": [Bson::Int64(0), Bson::Null] } }
        } else {
            doc! { "sender.commanders.secondary.id": secondary_commander_id }
        });
    }

    if let Some(exclusion_filter) = build_exclusion_filter(exclude_activities, exclude_battles) {
        and_filters.push(exclusion_filter);
    }

    doc! { "$and": and_filters }
}

fn pairings_projection() -> Document {
    doc! {
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
        "opponents.battle_results.sender.severely_wounded": 1,
        "opponents.battle_results.sender.heal": 1,
        "opponents.battle_results.sender.power": 1,
        "opponents.battle_results.opponent.kill_points": 1,
        "opponents.battle_results.opponent.severely_wounded": 1,
        "opponents.battle_results.opponent.slightly_wounded": 1,
        "opponents.battle_results.opponent.power": 1,
    }
}

fn build_exclusion_filter(
    exclude_activities: &[PairingsActivity],
    exclude_battles: &[PairingsBattleType],
) -> Option<Document> {
    let mut excluded_conditions = build_excluded_activity_conditions(exclude_activities);
    excluded_conditions.extend(build_excluded_battle_type_conditions(exclude_battles));

    (!excluded_conditions.is_empty()).then(|| doc! { "$nor": excluded_conditions })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_filter_matches_both_commanders_before_fetching() {
        let range = GovernorDateRange {
            start_millis: 1_000,
            end_millis: 2_000,
            start: "1970-01-01".to_string(),
            end: "1970-01-01".to_string(),
        };
        let filter = build_pairings_mail_filter(42, &range, Some((100, 200)), &[], &[]);
        let conditions = filter.get_array("$and").expect("conditions");
        assert!(
            conditions.contains(&Bson::Document(doc! { "sender.commanders.primary.id": 100_i64 }))
        );
        assert!(
            conditions
                .contains(&Bson::Document(doc! { "sender.commanders.secondary.id": 200_i64 }))
        );
    }

    #[test]
    fn pairings_filter_excludes_test_client_reports() {
        let range = GovernorDateRange {
            start_millis: 1_000,
            end_millis: 2_000,
            start: "1970-01-01".to_string(),
            end: "1970-01-01".to_string(),
        };

        let filter = build_pairings_mail_filter(42, &range, None, &[], &[]);
        let conditions = filter.get_array("$and").expect("pairings conditions");

        assert_eq!(
            conditions.first(),
            Some(&mongodb::bson::Bson::Document(
                doc! { "sender.app_id": { "$ne": 10_088_010_i64 } }
            ))
        );
    }

    #[test]
    fn exclusion_filter_is_absent_when_everything_is_included() {
        assert_eq!(build_exclusion_filter(&[], &[]), None);
    }

    #[test]
    fn exclusion_filter_combines_activities_and_battles() {
        let filter =
            build_exclusion_filter(&[PairingsActivity::Kvk], &[PairingsBattleType::Swarming]);

        assert_eq!(
            filter,
            Some(doc! {
                "$nor": [
                    doc! { "metadata.kvk": true },
                    build_excluded_battle_type_conditions(&[PairingsBattleType::Swarming])
                        .remove(0),
                ]
            })
        );
    }
}
