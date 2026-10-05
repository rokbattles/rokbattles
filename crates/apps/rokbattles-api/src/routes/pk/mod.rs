use std::sync::Arc;

use axum::{
    Json,
    extract::{
        FromRequestParts, Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{header::AUTHORIZATION, request::Parts},
    response::IntoResponse,
};
use futures::TryStreamExt;
use mongodb::bson::{Document, doc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{db::exclude_test_client_filter, error::ApiError, state::AppState};

pub(super) struct PkAuthorization;

impl FromRequestParts<Arc<AppState>> for PkAuthorization {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let expected = state
            .pk_token
            .as_deref()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(ApiError::unauthorized)?;

        let mut headers = parts.headers.get_all(AUTHORIZATION).iter();
        let (scheme, token) = headers
            .next()
            .and_then(|header| header.to_str().ok())
            .and_then(|header| header.split_once(' '))
            .ok_or_else(ApiError::unauthorized)?;

        if headers.next().is_some() || !scheme.eq_ignore_ascii_case("Bearer") || token.is_empty() {
            return Err(ApiError::unauthorized());
        }

        // Compare fixed-size digests so token contents and length do not affect comparison timing.
        let expected = Sha256::digest(expected.as_bytes());
        let actual = Sha256::digest(token.as_bytes());

        if !bool::from(expected.ct_eq(&actual)) {
            return Err(ApiError::unauthorized());
        }

        Ok(Self)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BattleReportsRequest {
    server_id: i64,
    open_time: i64,
    close_time: i64,
}

impl BattleReportsRequest {
    fn filter(&self) -> Result<Document, ApiError> {
        if self.server_id <= 0 {
            return Err(ApiError::bad_request("serverId must be positive"));
        }

        if self.open_time <= 0 || self.close_time < self.open_time {
            return Err(ApiError::bad_request(
                "openTime must be greater than zero and closeTime must be greater than or equal to openTime",
            ));
        }

        let to_mail_time = |millis: i64| {
            millis.checked_mul(1000).ok_or_else(|| {
                ApiError::bad_request("openTime and closeTime must fit in microseconds")
            })
        };

        let open_time = to_mail_time(self.open_time)?;
        let close_time = to_mail_time(self.close_time)?;

        let mut filter = exclude_test_client_filter();
        filter.extend(doc! {
            "metadata.server_id": self.server_id,
            "metadata.mail_time": { "$gte": open_time, "$lte": close_time },
            "sender.player_id": { "$gt": 0 },
            "opponents.player_id": { "$gt": 0 },
        });

        Ok(filter)
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(super) struct Pagination {
    page: u64,
    size: u32,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page: 1, size: 100 }
    }
}

impl Pagination {
    fn offset(&self) -> Result<u64, ApiError> {
        if self.page == 0 || !(1..=1000).contains(&self.size) {
            return Err(ApiError::bad_request(
                "page must be at least 1 and size must be between 1 and 1000",
            ));
        }

        (self.page - 1)
            .checked_mul(u64::from(self.size))
            .filter(|offset| i64::try_from(*offset).is_ok())
            .ok_or_else(|| ApiError::bad_request("page is too large"))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BattleReportsResponse {
    items: Vec<Document>,
    page: u64,
    size: u32,
    has_more: bool,
}

pub(super) async fn battle_reports(
    State(state): State<Arc<AppState>>,
    _authorization: PkAuthorization,
    pagination: Result<Query<Pagination>, QueryRejection>,
    request: Result<Json<BattleReportsRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(pagination) = pagination.map_err(|error| ApiError::bad_request(error.body_text()))?;
    let Json(request) = request.map_err(|error| ApiError::bad_request(error.body_text()))?;

    let offset = pagination.offset()?;
    let filter = request.filter()?;

    let mut items: Vec<Document> = state
        .reports_store
        .battle_collection()
        .find(filter)
        .sort(doc! { "metadata.mail_time": 1, "_id": 1 })
        .skip(offset)
        .limit(i64::from(pagination.size) + 1)
        .projection(doc! { "_id": 0 })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .try_collect()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let has_more = items.len() > pagination.size as usize;
    items.truncate(pagination.size as usize);

    let response =
        BattleReportsResponse { items, page: pagination.page, size: pagination.size, has_more };

    Ok(([("Cache-Control", "no-store")], Json(response)))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn valid_body() -> Value {
        json!({ "serverId": 16053, "openTime": 1_700_000_000_123_i64, "closeTime": 1_700_000_000_133_i64 })
    }

    #[test]
    fn converts_lostland_milliseconds_to_exact_mail_microseconds() {
        let request: BattleReportsRequest = serde_json::from_value(valid_body()).expect("request");
        let filter = request.filter().expect("filter");

        assert_eq!(
            filter.get_document("metadata.mail_time").expect("time bounds"),
            &doc! {
                "$gte": 1_700_000_000_123_000_i64,
                "$lte": 1_700_000_000_133_000_i64,
            }
        );
    }
}
