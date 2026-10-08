use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::TryStreamExt;
use mongodb::{
    bson::{Document, doc},
    options::FindOptions,
};
use serde::Deserialize;

use super::{
    mapper::map_match_record,
    matcher::match_ark_mails,
    query::parse_match_id,
    store::fetch_ark_battle_results_mail_by_id,
    types::{ArkLeague, ArkMatchSummary, ArkReportCoverage, ArkReportStatus, ArkReportsResponse},
};
use crate::{
    auth::AuthenticatedSession,
    db::exclude_test_client_filter,
    error::ApiError,
    routes::{
        governor::{
            common::{ensure_governor_claim_for_user, parse_governor_id_param},
            store_utils::fetch_collection_documents,
        },
        reports::battle::list_mapper::{build_battle_list_projection, map_battle_list_document},
    },
    state::AppState,
    time_utils::build_mail_time_match,
};

const PAGE_SIZE: u64 = 20;
const BEFORE_RESULT_MILLIS: i64 = 65 * 60 * 1000;
const AFTER_RESULT_MILLIS: i64 = 2 * 60 * 1000;

pub(crate) struct ReportScope {
    pub filter: Option<Document>,
    pub coverage: ArkReportCoverage,
    pub league: Option<ArkLeague>,
}

#[derive(Deserialize)]
pub struct ReportsPage {
    page: Option<u64>,
}

/// Lists uploaded battles from a single unambiguous Ark session owned by the governor.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Path((governor_id_raw, match_id_raw)): Path<(String, String)>,
    Query(query): Query<ReportsPage>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let governor_id = parse_governor_id_param(&governor_id_raw)?;
    let match_id = parse_match_id(&match_id_raw)?;
    let page = query.page.unwrap_or(1).clamp(1, 10_000);

    ensure_governor_claim_for_user(&state, &session.user.discord_id, governor_id).await?;

    let receiver = format!("player_{governor_id}");
    let mail = fetch_ark_battle_results_mail_by_id(&state, &receiver, &match_id).await?;
    let summary = mail.and_then(|mail| {
        match_ark_mails(vec![mail], Vec::new(), super::MATCH_DELTA_MILLIS)
            .first()
            .map(|entry| map_match_record(entry, 0))
    });

    let mut items = Vec::new();
    let mut coverage = ArkReportCoverage { status: ArkReportStatus::Missing, total: 0 };

    if let Some(summary) = summary {
        let scope = resolve_report_scope(&state, &receiver, &summary).await?;
        coverage = scope.coverage;

        if let Some(filter) = scope.filter {
            let options = FindOptions::builder()
                .sort(doc! { "metadata.mail_time": -1, "metadata.mail_id": -1 })
                .skip((page - 1) * PAGE_SIZE)
                .limit(PAGE_SIZE as i64)
                .projection(build_battle_list_projection())
                .build();

            let documents: Vec<Document> = fetch_collection_documents(
                state.reports_store.battle_collection(),
                filter,
                options,
            )
            .await?;

            items =
                documents.iter().filter_map(map_battle_list_document).map(|row| row.item).collect();
        }
    }

    let response = ArkReportsResponse { coverage, page, page_size: PAGE_SIZE, items };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

pub(crate) async fn resolve_report_scope(
    state: &Arc<AppState>,
    receiver: &str,
    summary: &ArkMatchSummary,
) -> Result<ReportScope, ApiError> {
    let Some(mut filter) = report_candidates_filter(receiver, summary) else {
        return Ok(scope_from_sessions(None, &[]));
    };

    let sessions: Vec<Document> = state
        .reports_store
        .battle_collection()
        .aggregate(vec![
            doc! { "$match": filter.clone() },
            doc! { "$group": { "_id": "$sender.session", "total": { "$sum": 1 } } },
            // Two sessions are enough to establish ambiguity; never guess between them.
            doc! { "$limit": 2 },
        ])
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .try_collect()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut scope = scope_from_sessions(Some(summary.league), &sessions);
    if scope.coverage.status == ArkReportStatus::Available
        && let Some(session) = sessions.first().and_then(|value| value.get_str("_id").ok())
    {
        filter.insert("sender.session", session);
        scope.filter = Some(filter);
    }

    Ok(scope)
}

fn scope_from_sessions(mail_league: Option<ArkLeague>, sessions: &[Document]) -> ReportScope {
    let mut scope = ReportScope {
        filter: None,
        coverage: ArkReportCoverage { status: ArkReportStatus::Missing, total: 0 },
        league: None,
    };

    let [session] = sessions else {
        if sessions.len() > 1 {
            scope.coverage.status = ArkReportStatus::Ambiguous;
        }

        return scope;
    };

    let league = session.get_str("_id").ok().and_then(league_from_session);
    if league.is_none()
        || mail_league.is_some_and(|known| known != ArkLeague::Unknown && Some(known) != league)
    {
        scope.coverage.status = ArkReportStatus::Ambiguous;

        return scope;
    }

    scope.league = league;
    scope.coverage.status = ArkReportStatus::Available;
    scope.coverage.total =
        rokbattles_bson::bson_to_i64(session.get("total").unwrap_or(&mongodb::bson::Bson::Null))
            .and_then(|total| u64::try_from(total).ok())
            .unwrap_or(0);

    scope
}

fn report_candidates_filter(receiver: &str, summary: &ArkMatchSummary) -> Option<Document> {
    let ids = summary.alliances.iter().filter_map(|alliance| alliance.id).collect::<Vec<_>>();
    let [first, second] = ids.as_slice() else {
        return None;
    };

    if first == second || *first <= 0 || *second <= 0 {
        return None;
    }

    let start = summary.mail_time_millis.saturating_sub(BEFORE_RESULT_MILLIS);
    let end = summary.mail_time_millis.saturating_add(AFTER_RESULT_MILLIS);

    Some(doc! {
        "$and": [
            exclude_test_client_filter(),
            build_mail_time_match(start, end),
            { "metadata.mail_receiver": receiver, "metadata.mail_role": "dungeon" },
            // Enforce both opposing alliances, including the same opponent's player ID.
            { "$or": [
                { "sender.alliance.id": first, "opponents": { "$elemMatch": { "alliance.id": second, "player_id": { "$gt": 0 } } } },
                { "sender.alliance.id": second, "opponents": { "$elemMatch": { "alliance.id": first, "player_id": { "$gt": 0 } } } },
            ] },
        ]
    })
}

fn league_from_session(session: &str) -> Option<ArkLeague> {
    let parameter = |key: &str| {
        session
            .split('&')
            .filter_map(|part| part.split_once('='))
            .find_map(|(name, value)| (name == key).then_some(value))
    };

    let mode = parameter("mode")?;
    let submode = parameter("submode");

    match (mode, submode) {
        ("abl", _) => Some(ArkLeague::Osiris),
        ("ab", Some("SilverEgypt")) => Some(ArkLeague::Silver),
        ("ab", Some("")) => Some(ArkLeague::Golden),
        ("abp", Some("gvgn")) => Some(ArkLeague::Practice),
        ("abp", Some("DiyEgypt")) => Some(ArkLeague::Custom),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{super::types::ArkMatchAlliance, *};

    #[test]
    fn classifies_only_exact_ark_session_parameters() {
        for (session, expected) in [
            ("cfg_id=8&id=3321&mode=ab&role_id=124&submode=", Some(ArkLeague::Golden)),
            ("mode=ab&submode=SilverEgypt", Some(ArkLeague::Silver)),
            ("mode=abl&id=99", Some(ArkLeague::Osiris)),
            ("mode=abp&submode=gvgn", Some(ArkLeague::Practice)),
            ("mode=abp&submode=DiyEgypt", Some(ArkLeague::Custom)),
            ("othermode=ab&submode=", None),
            ("mode=ab", None),
            ("mode=ab&submode=other", None),
        ] {
            assert_eq!(league_from_session(session), expected, "{session}");
        }
    }

    #[test]
    fn does_not_mix_adjacent_sessions_or_conflicting_leagues() {
        let golden = doc! { "_id": "mode=ab&submode=", "total": 20 };
        let silver = doc! { "_id": "mode=ab&submode=SilverEgypt", "total": 10 };

        assert_eq!(scope_from_sessions(None, &[]).coverage.status, ArkReportStatus::Missing);
        assert_eq!(
            scope_from_sessions(None, &[golden.clone(), silver]).coverage.status,
            ArkReportStatus::Ambiguous
        );
        assert_eq!(
            scope_from_sessions(Some(ArkLeague::Osiris), std::slice::from_ref(&golden))
                .coverage
                .status,
            ArkReportStatus::Ambiguous
        );

        let scope = scope_from_sessions(Some(ArkLeague::Unknown), &[golden]);
        assert_eq!(scope.coverage.total, 20);
        assert_eq!(scope.league, Some(ArkLeague::Golden));
    }

    #[test]
    fn candidates_require_receiver_two_distinct_alliances_and_bounded_time() {
        let alliance = |id| ArkMatchAlliance {
            id: Some(id),
            name: None,
            logo: None,
            abbreviation: None,
            score: None,
            members: None,
            members_max: None,
            is_blue: None,
        };
        let mut summary = ArkMatchSummary {
            match_id: "match".into(),
            mail_time_millis: 1_790_000_000_000,
            alliances: vec![alliance(42), alliance(43)],
            winner_alliance_id: None,
            self_alliance_id: Some(42),
            league: ArkLeague::Unknown,
            personal_score: None,
            has_individual_results: false,
        };

        let filter = report_candidates_filter("player_7", &summary).expect("filter");
        let conditions = filter.get_array("$and").expect("conditions");

        assert_eq!(conditions[1], doc! { "metadata.mail_time": { "$gte": 1_789_996_100_000_000_i64, "$lt": 1_790_000_120_000_000_i64 } }.into());
        assert_eq!(
            conditions[2],
            doc! { "metadata.mail_receiver": "player_7", "metadata.mail_role": "dungeon" }.into()
        );

        let pair_filter =
            conditions[3].as_document().expect("pair").get_array("$or").expect("directions");
        assert_eq!(pair_filter.len(), 2);

        summary.alliances[1].id = Some(42);
        assert!(report_candidates_filter("player_7", &summary).is_none());
    }
}
