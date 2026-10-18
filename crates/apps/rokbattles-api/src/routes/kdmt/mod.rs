use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State, rejection::QueryRejection},
    response::IntoResponse,
    routing::get,
};
use futures::TryStreamExt;
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc, oid::ObjectId},
};
use serde::{Deserialize, Serialize};

use self::{auth::KdmtAuthorization, cursor::Cursor};
use crate::{
    db::{ReportsStore, exclude_test_reports_filter},
    error::ApiError,
    state::AppState,
};

mod auth;
mod cursor;

const KINGDOM_ID: i64 = 1821;
const WINDOW_MILLIS: i64 = 90 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Feed {
    Battles,
    Resources,
    Barbarians,
    BarbarianForts,
    Baulurs,
    KaruakCeremony,
    KaharsTreasure,
    EliteBarbarians,
}

impl Feed {
    const ALL: [Self; 8] = [
        Self::Battles,
        Self::Resources,
        Self::Barbarians,
        Self::BarbarianForts,
        Self::Baulurs,
        Self::KaruakCeremony,
        Self::KaharsTreasure,
        Self::EliteBarbarians,
    ];

    fn path(self) -> &'static str {
        match self {
            Self::Battles => "/battle-reports",
            Self::Resources => "/resources",
            Self::Barbarians => "/loot/barbarians",
            Self::BarbarianForts => "/loot/barbarian-forts",
            Self::Baulurs => "/loot/baulurs",
            Self::KaruakCeremony => "/loot/karuak-ceremony",
            Self::KaharsTreasure => "/loot/kahars-treasure",
            Self::EliteBarbarians => "/loot/elite-barbarians",
        }
    }

    fn collection(self, store: &ReportsStore) -> &Collection<Document> {
        match self {
            Self::Battles | Self::Barbarians => store.battle_collection(),
            Self::Resources => store.rss_collection(),
            Self::BarbarianForts => store.system_barbarian_fort_collection(),
            Self::Baulurs => store.barcanyonkillboss_collection(),
            Self::KaruakCeremony => store.event_member_loot_report_collection(),
            Self::KaharsTreasure => store.system_kahar_treasure_collection(),
            Self::EliteBarbarians => store.kill_elite_bar_report_collection(),
        }
    }

    fn has_participants(self) -> bool {
        matches!(self, Self::Baulurs | Self::KaruakCeremony | Self::EliteBarbarians)
    }
}

pub(super) fn router() -> Router<Arc<AppState>> {
    let mut router = Router::new();

    for feed in Feed::ALL {
        router = router
            .route(feed.path(), get(move |state, auth, query| list(state, auth, query, feed)));
    }

    router
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    player_id: Option<i64>,
    #[serde(default = "default_size")]
    size: u32,
    cursor: Option<String>,
}

fn default_size() -> u32 {
    100
}

impl Request {
    fn validate(&self, feed: Feed, now: i64) -> Result<Option<ObjectId>, ApiError> {
        if !(1..=1000).contains(&self.size) {
            return Err(ApiError::bad_request("size must be between 1 and 1000"));
        }

        if self.player_id.is_some_and(|id| id <= 0) {
            return Err(ApiError::bad_request("playerId must be positive"));
        }

        if feed != Feed::Battles && self.player_id.is_none() {
            return Err(ApiError::bad_request("playerId is required for resources and loot"));
        }

        self.cursor
            .as_deref()
            .map(|cursor| Cursor::decode(cursor, feed, self.player_id, now))
            .transpose()
            .map(Option::flatten)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    items: Vec<Document>,
    size: u32,
    has_more: bool,
    next_cursor: String,
}

async fn list(
    State(state): State<Arc<AppState>>,
    _authorization: KdmtAuthorization,
    query: Result<Query<Request>, QueryRejection>,
    feed: Feed,
) -> Result<impl IntoResponse, ApiError> {
    let Query(request) = query.map_err(|error| ApiError::bad_request(error.body_text()))?;
    let now = DateTime::now().timestamp_millis();
    let after = request.validate(feed, now)?;

    let response = fetch_page(&state.reports_store, feed, &request, after, now).await?;

    Ok(([("Cache-Control", "no-store")], Json(response)))
}

fn time_filter(now: i64) -> Document {
    doc! { "metadata.mail_time": {
        "$gte": (now - WINDOW_MILLIS) * 1000,
        "$lte": now * 1000,
    } }
}

async fn ensure_membership(store: &ReportsStore, player_id: i64, now: i64) -> Result<(), ApiError> {
    let latest = store
        .battle_collection()
        .find_one(doc! { "$and": [
            exclude_test_reports_filter(),
            { "sender.player_id": player_id },
            time_filter(now),
        ] })
        .sort(doc! { "metadata.mail_time": -1, "_id": -1 })
        .projection(doc! { "sender.kingdom_id": 1 })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let kingdom = latest
        .as_ref()
        .and_then(|report| report.get_document("sender").ok())
        .and_then(|sender| sender.get("kingdom_id"))
        .and_then(integer);

    if kingdom != Some(KINGDOM_ID) {
        return Err(ApiError::Forbidden(
            "Player has no recent eligible sender report in kingdom 1821".into(),
        ));
    }

    Ok(())
}

fn integer(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        _ => None,
    }
}

fn report_filter(
    feed: Feed,
    player_id: Option<i64>,
    after: Option<ObjectId>,
    now: i64,
) -> Document {
    let mut filter = exclude_test_reports_filter();
    filter.extend(time_filter(now));

    filter
        .insert("_id", after.map_or_else(|| doc! { "$type": "objectId" }, |id| doc! { "$gt": id }));

    if feed == Feed::Battles {
        filter.insert("sender.kingdom_id", KINGDOM_ID);
        filter.insert(
            "sender.player_id",
            player_id.map_or_else(|| Bson::Document(doc! { "$gt": 0 }), Bson::Int64),
        );
    } else if let Some(player_id) = player_id {
        filter.insert("metadata.mail_receiver", format!("player_{player_id}"));

        if feed.has_participants() {
            filter.insert("participants.player_id", player_id);
        }

        if feed == Feed::Barbarians {
            filter.insert("sender.player_id", player_id);
            filter.insert("sender.kingdom_id", KINGDOM_ID);
            filter.insert("opponents.npc.loot.0", doc! { "$exists": true });
        }
    }

    filter
}

async fn fetch_page(
    store: &ReportsStore,
    feed: Feed,
    request: &Request,
    after: Option<ObjectId>,
    now: i64,
) -> Result<Response, ApiError> {
    if feed != Feed::Battles {
        let player_id =
            request.player_id.ok_or_else(|| ApiError::bad_request("playerId is required"))?;
        ensure_membership(store, player_id, now).await?;
    }

    let mut items: Vec<Document> = feed
        .collection(store)
        .find(report_filter(feed, request.player_id, after, now))
        .sort(doc! { "_id": 1 })
        .limit(i64::from(request.size) + 1)
        .projection(doc! { "metadata.source_checksum": 0, "metadata.source_size": 0 })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .try_collect()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let has_more = items.len() > request.size as usize;
    items.truncate(request.size as usize);

    let mut last_id = after;

    for item in &mut items {
        let id =
            item.get_object_id("_id").map_err(|error| ApiError::internal(error.to_string()))?;
        last_id = Some(id);
        item.remove("_id");

        if feed.has_participants()
            && let Ok(participants) = item.get_array_mut("participants")
        {
            participants.retain(|participant| {
                participant.as_document().and_then(|row| row.get("player_id")).and_then(integer)
                    == request.player_id
            });
        }
    }

    Ok(Response {
        items,
        size: request.size,
        has_more,
        next_cursor: Cursor::encode(feed, request.player_id, last_id, now)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(player_id: Option<i64>, size: u32, cursor: Option<String>) -> Request {
        Request { player_id, size, cursor }
    }

    #[test]
    fn validates_size_player_and_unknown_parameters() {
        let now = DateTime::now().timestamp_millis();

        for feed in Feed::ALL {
            for size in [1, 100, 1000] {
                request(Some(42), size, None).validate(feed, now).unwrap();
            }

            for size in [0, 1001, u32::MAX] {
                request(Some(42), size, None).validate(feed, now).unwrap_err();
            }

            for player_id in [0, -1] {
                request(Some(player_id), 100, None).validate(feed, now).unwrap_err();
            }

            assert_eq!(request(None, 100, None).validate(feed, now).is_ok(), feed == Feed::Battles);
        }

        assert_eq!(serde_json::from_str::<Request>("{}").unwrap().size, 100);
        serde_json::from_str::<Request>(r#"{"serverId":1821}"#).unwrap_err();
        serde_json::from_str::<Request>(r#"{"openTime":1}"#).unwrap_err();
    }
}
