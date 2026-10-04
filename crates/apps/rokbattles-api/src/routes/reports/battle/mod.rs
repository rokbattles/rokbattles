use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::StreamExt;
use mongodb::{
    bson::{Bson, doc},
    options::{FindOneOptions, FindOptions, Hint},
};
use rustc_hash::{FxHashMap, FxHashSet};

use self::{
    detail_mapper::{
        build_battle_detail_filter, build_battle_detail_projection, map_battle_detail_document,
    },
    list_mapper::{build_report_dedupe_key, map_battle_list_document},
    match_builder::build_reports_match,
    query::{ReportsFilterSide, ReportsFilterType, ReportsRequest, parse_reports_request},
    types::{ReportByIdResponse, ReportRowWithCursor, ReportsResponse},
};
use crate::{
    error::ApiError, routes::reports::common::pagination::paginate_cursor_rows, state::AppState,
};

mod detail_mapper;
mod list_mapper;
mod map_context;
mod match_builder;
mod query;
mod stratagems;
mod structure_override;
mod types;

const PAGE_SIZE: usize = 100;
const FETCH_LIMIT: i64 = PAGE_SIZE as i64 + 1;
// 1 month
const REPORT_DETAIL_CACHE_CONTROL: &str = "public, max-age=2592000";

/// List battle reports with filters and cursor pagination.
pub async fn get(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FxHashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_reports_request(&params)?;
    let final_match = build_reports_match(&request);

    let options = build_battle_list_find_options(&request);

    let mut cursor = state
        .reports_store
        .battle_collection()
        .find(final_match)
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut dedupe_keys = FxHashSet::default();
    let mut rows: Vec<ReportRowWithCursor> = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        let Some(row) = map_battle_list_document(&document) else {
            continue;
        };
        let dedupe_key = build_report_dedupe_key(&document)
            .unwrap_or_else(|| format!("mail:{}", row.item.mail_id));
        if dedupe_keys.insert(dedupe_key) {
            rows.push(row);
            if rows.len() >= FETCH_LIMIT as usize {
                break;
            }
        }
    }

    let mut paged_rows = paginate_cursor_rows(
        rows,
        dedupe_keys.len(),
        PAGE_SIZE,
        request.before_cursor,
        request.after_cursor,
        |row: &ReportRowWithCursor| row.mail_time,
    );

    map_context::enrich_map_context(&state.reports_store, &mut paged_rows.items).await?;

    let response = ReportsResponse {
        items: paged_rows.items.into_iter().map(|row| row.item).collect(),
        next_after: paged_rows.next_after,
        previous_before: paged_rows.previous_before,
    };

    Ok((StatusCode::OK, [("Cache-Control", "no-store")], Json(response)))
}

fn build_battle_list_find_options(request: &ReportsRequest) -> FindOptions {
    let hint = battle_list_index_hint(request);

    // Explicit bounds keep partial-index scans from starting outside the requested page.
    let cursor_index_field = if should_hint_home_partial_index(request) {
        Some("metadata.kvk")
    } else if request.garrison_building_type.is_none() && hint.is_some() {
        match request.garrison_side {
            ReportsFilterSide::Sender => Some("sender.structure_id"),
            ReportsFilterSide::Opponent => Some("opponents.structure_id"),
            ReportsFilterSide::None | ReportsFilterSide::Both => None,
        }
    } else {
        None
    };

    let max = request.before_cursor.zip(cursor_index_field).map(|(before_cursor, field)| {
        doc! { "metadata.mail_time": before_cursor, field: Bson::MinKey }
    });

    let min = request.after_cursor.zip(cursor_index_field).map(|(after_cursor, field)| {
        doc! { "metadata.mail_time": after_cursor, field: Bson::MaxKey }
    });

    FindOptions::builder()
        .sort(doc! { "metadata.mail_time": request.sort_direction() })
        .projection(self::list_mapper::build_battle_list_projection())
        .hint(hint)
        .max(max)
        .min(min)
        .build()
}

fn battle_list_index_hint(request: &ReportsRequest) -> Option<Hint> {
    if should_hint_home_partial_index(request) {
        return Some(Hint::Keys(doc! { "metadata.mail_time": -1, "metadata.kvk": 1 }));
    }

    if request.player_id.is_some()
        || request.sender_primary_commander_id.is_some()
        || request.sender_secondary_commander_id.is_some()
        || request.opponent_primary_commander_id.is_some()
        || request.opponent_secondary_commander_id.is_some()
    {
        return None;
    }

    // Alliance-building partial indexes omit structure-only garrisons.
    match (request.garrison_side, request.garrison_building_type) {
        (ReportsFilterSide::Sender, Some(_)) => {
            return Some(Hint::Keys(doc! {
                "metadata.mail_time": -1,
                "sender.alliance_building_id": 1,
            }));
        }

        (ReportsFilterSide::Opponent, Some(_)) => {
            return Some(Hint::Keys(doc! {
                "metadata.mail_time": -1,
                "opponents.alliance_building_id": 1,
            }));
        }

        (ReportsFilterSide::Sender, None) => {
            return Some(Hint::Keys(doc! {
                "metadata.mail_time": -1,
                "sender.structure_id": 1,
            }));
        }

        (ReportsFilterSide::Opponent, None) => {
            return Some(Hint::Keys(doc! {
                "metadata.mail_time": -1,
                "opponents.structure_id": 1,
            }));
        }

        (ReportsFilterSide::None, _) => {}
        (ReportsFilterSide::Both, _) => return None,
    }

    match request.rally_side {
        ReportsFilterSide::Sender => {
            return Some(Hint::Keys(doc! {
                "metadata.mail_time": -1,
                "sender.rally": 1,
            }));
        }
        ReportsFilterSide::Opponent | ReportsFilterSide::Both => return None,
        ReportsFilterSide::None => {}
    }

    matches!(request.filter_type, Some(ReportsFilterType::Kvk)).then(|| {
        Hint::Keys(doc! {
            "metadata.mail_time": -1,
            "metadata.kvk": 1,
            "opponents.player_id": 1,
        })
    })
}

fn should_hint_home_partial_index(request: &ReportsRequest) -> bool {
    matches!(request.filter_type, Some(ReportsFilterType::Home))
        && request.player_id.is_none()
        && request.sender_primary_commander_id.is_none()
        && request.sender_secondary_commander_id.is_none()
        && request.opponent_primary_commander_id.is_none()
        && request.opponent_secondary_commander_id.is_none()
        && matches!(request.rally_side, ReportsFilterSide::None)
        && matches!(request.garrison_side, ReportsFilterSide::None)
}

/// Look up a single battle report by mail ID.
pub async fn get_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let report_id = parse_report_id(&id)?;

    let options = FindOneOptions::builder().projection(build_battle_detail_projection()).build();

    let mail = state
        .reports_store
        .battle_collection()
        .find_one(build_battle_detail_filter(&report_id))
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let response = ReportByIdResponse {
        id: report_id,
        mail: mail.as_ref().and_then(map_battle_detail_document),
    };

    Ok((StatusCode::OK, [("Cache-Control", REPORT_DETAIL_CACHE_CONTROL)], Json(response)))
}

fn parse_report_id(raw_id: &str) -> Result<String, ApiError> {
    let normalized = raw_id.trim();
    if normalized.is_empty() {
        return Err(ApiError::bad_request("Invalid report id"));
    }

    Ok(normalized.to_string())
}

#[cfg(test)]
mod tests {
    use mongodb::{
        bson::{Bson, doc},
        options::Hint,
    };
    use rustc_hash::FxHashMap;

    use super::{build_battle_list_find_options, parse_report_id};
    use crate::routes::reports::battle::query::parse_reports_request;

    #[test]
    fn home_list_options_hint_the_home_partial_index() {
        let request = parse_reports_request(&FxHashMap::from_iter([(
            "type".to_string(),
            "home".to_string(),
        )]))
        .expect("valid Home filter");

        let options = build_battle_list_find_options(&request);

        assert!(matches!(
            options.hint,
            Some(Hint::Keys(keys))
                if keys == doc! { "metadata.mail_time": -1, "metadata.kvk": 1 }
        ));
        assert!(options.max.is_none());
        assert!(options.min.is_none());
    }

    #[test]
    fn home_before_cursor_bounds_the_home_partial_index() {
        let request = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "home".to_string()),
            ("before".to_string(), "123456".to_string()),
        ]))
        .expect("valid Home before cursor");

        let options = build_battle_list_find_options(&request);

        assert_eq!(
            options.max,
            Some(doc! {
                "metadata.mail_time": 123456_i64,
                "metadata.kvk": Bson::MinKey,
            })
        );
        assert!(options.min.is_none());
    }

    #[test]
    fn home_after_cursor_bounds_the_home_partial_index() {
        let request = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "home".to_string()),
            ("after".to_string(), "123456".to_string()),
        ]))
        .expect("valid Home after cursor");

        let options = build_battle_list_find_options(&request);

        assert_eq!(
            options.min,
            Some(doc! {
                "metadata.mail_time": 123456_i64,
                "metadata.kvk": Bson::MaxKey,
            })
        );
        assert!(options.max.is_none());
    }

    #[test]
    fn home_list_options_allow_specialized_commander_index() {
        let request = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "home".to_string()),
            ("ssc".to_string(), "618".to_string()),
        ]))
        .expect("valid Home commander filter");

        let options = build_battle_list_find_options(&request);

        assert!(options.hint.is_none());
        assert!(options.max.is_none());
        assert!(options.min.is_none());
    }

    #[test]
    fn kvk_list_options_hint_the_human_kvk_index() {
        assert_filter_uses_hint(
            "type",
            "kvk",
            doc! {
                "metadata.mail_time": -1,
                "metadata.kvk": 1,
                "opponents.player_id": 1,
            },
        );
    }

    #[test]
    fn sender_rally_list_options_hint_the_sender_rally_index() {
        assert_filter_uses_hint(
            "rs",
            "sender",
            doc! { "metadata.mail_time": -1, "sender.rally": 1 },
        );
    }

    #[test]
    fn any_garrison_list_options_hint_indexes_that_include_structures() {
        for (side, field) in
            [("sender", "sender.structure_id"), ("opponent", "opponents.structure_id")]
        {
            let request = parse_reports_request(&FxHashMap::from_iter([(
                "gs".to_string(),
                side.to_string(),
            )]))
            .expect("valid garrison filter");

            assert!(matches!(
                build_battle_list_find_options(&request).hint,
                Some(Hint::Keys(keys)) if keys == doc! { "metadata.mail_time": -1, field: 1 }
            ));
        }
    }

    #[test]
    fn garrison_before_cursor_bounds_the_structure_index() {
        for (side, field) in
            [("sender", "sender.structure_id"), ("opponent", "opponents.structure_id")]
        {
            let request = parse_reports_request(&FxHashMap::from_iter([
                ("gs".to_string(), side.to_string()),
                ("before".to_string(), "123456".to_string()),
            ]))
            .expect("valid garrison before cursor");

            let options = build_battle_list_find_options(&request);

            assert_eq!(
                options.max,
                Some(doc! { "metadata.mail_time": 123456_i64, field: Bson::MinKey }),
            );
            assert!(options.min.is_none());
        }
    }

    #[test]
    fn garrison_after_cursor_bounds_the_structure_index() {
        for (side, field) in
            [("sender", "sender.structure_id"), ("opponent", "opponents.structure_id")]
        {
            let request = parse_reports_request(&FxHashMap::from_iter([
                ("gs".to_string(), side.to_string()),
                ("after".to_string(), "123456".to_string()),
            ]))
            .expect("valid garrison after cursor");

            let options = build_battle_list_find_options(&request);

            assert_eq!(
                options.min,
                Some(doc! { "metadata.mail_time": 123456_i64, field: Bson::MaxKey }),
            );
            assert!(options.max.is_none());
        }
    }

    #[test]
    fn both_garrison_list_options_do_not_hint_a_single_side_index() {
        let request =
            parse_reports_request(&FxHashMap::from_iter([("gs".to_string(), "both".to_string())]))
                .expect("valid garrison filter");

        assert!(build_battle_list_find_options(&request).hint.is_none());
    }

    #[test]
    fn garrison_list_options_allow_specialized_player_and_commander_indexes() {
        for side in ["sender", "opponent"] {
            for (parameter, value) in [
                ("pid", "87102301"),
                ("spc", "503"),
                ("ssc", "184"),
                ("opc", "503"),
                ("osc", "184"),
            ] {
                let request = parse_reports_request(&FxHashMap::from_iter([
                    ("gs".to_string(), side.to_string()),
                    (parameter.to_string(), value.to_string()),
                    ("before".to_string(), "123456".to_string()),
                ]))
                .expect("valid garrison player or commander filter");

                let options = build_battle_list_find_options(&request);

                assert!(options.hint.is_none());
                assert!(options.max.is_none());
                assert!(options.min.is_none());
            }
        }
    }

    #[test]
    fn specific_garrison_buildings_keep_alliance_index_hints() {
        for (side, field) in [
            ("sender", "sender.alliance_building_id"),
            ("opponent", "opponents.alliance_building_id"),
        ] {
            for building in ["flag", "fortress", "other"] {
                let request = parse_reports_request(&FxHashMap::from_iter([
                    ("gs".to_string(), side.to_string()),
                    ("gb".to_string(), building.to_string()),
                ]))
                .expect("valid garrison building filter");

                assert!(matches!(
                    build_battle_list_find_options(&request).hint,
                    Some(Hint::Keys(keys)) if keys == doc! { "metadata.mail_time": -1, field: 1 }
                ));
            }
        }
    }

    #[test]
    fn specialized_list_options_allow_player_index() {
        let request = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "kvk".to_string()),
            ("pid".to_string(), "123".to_string()),
        ]))
        .expect("valid KVK player filter");

        assert!(build_battle_list_find_options(&request).hint.is_none());
    }

    #[test]
    fn kvk_opponent_rally_options_allow_existing_rally_index() {
        let request = parse_reports_request(&FxHashMap::from_iter([
            ("type".to_string(), "kvk".to_string()),
            ("rs".to_string(), "opponent".to_string()),
        ]))
        .expect("valid KVK opponent rally filter");

        assert!(build_battle_list_find_options(&request).hint.is_none());
    }

    fn assert_filter_uses_hint(parameter: &str, value: &str, expected: mongodb::bson::Document) {
        let request = parse_reports_request(&FxHashMap::from_iter([(
            parameter.to_string(),
            value.to_string(),
        )]))
        .expect("valid report filter");

        assert!(matches!(
            build_battle_list_find_options(&request).hint,
            Some(Hint::Keys(keys)) if keys == expected
        ));
    }

    #[test]
    fn parses_non_empty_report_id() {
        let parsed = parse_report_id("mail-123").expect("id should parse");
        assert_eq!(parsed, "mail-123");
    }

    #[test]
    fn trims_and_rejects_empty_report_id() {
        assert_eq!(parse_report_id("  mail-123  ").expect("id should parse"), "mail-123");
        parse_report_id("   ").expect_err("input should be rejected");
    }
}
