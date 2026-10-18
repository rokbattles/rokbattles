use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::{Feed, WINDOW_MILLIS};
use crate::error::ApiError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cursor {
    version: u8,
    feed: Feed,
    player_id: Option<i64>,
    after: Option<String>,
    issued_at: i64,
}

impl Cursor {
    pub fn encode(
        feed: Feed,
        player_id: Option<i64>,
        after: Option<ObjectId>,
        now: i64,
    ) -> Result<String, ApiError> {
        let cursor = Self {
            version: 1,
            feed,
            player_id,
            after: after.map(|id| id.to_hex()),
            issued_at: now,
        };

        let json =
            serde_json::to_vec(&cursor).map_err(|error| ApiError::internal(error.to_string()))?;

        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode(
        value: &str,
        feed: Feed,
        player_id: Option<i64>,
        now: i64,
    ) -> Result<Option<ObjectId>, ApiError> {
        let invalid = || ApiError::bad_request("Invalid cursor for this endpoint/playerId");

        if value.len() > 1024 {
            return Err(invalid());
        }

        let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_error| invalid())?;
        let cursor: Self = serde_json::from_slice(&bytes).map_err(|_error| invalid())?;

        if cursor.version != 1
            || cursor.feed != feed
            || cursor.player_id != player_id
            || cursor.issued_at <= 0
            || cursor.issued_at > now
        {
            return Err(invalid());
        }

        if cursor.issued_at < now - WINDOW_MILLIS {
            return Err(ApiError::Gone(
                "Cursor expired; restart without a cursor to scan the available 90 days".into(),
            ));
        }

        cursor.after.map(|id| ObjectId::parse_str(id).map_err(|_error| invalid())).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    #[test]
    fn roundtrips_empty_and_nonempty_bookmarks_and_rejects_wrong_scopes() {
        for id in [None, Some(ObjectId::new())] {
            let value = Cursor::encode(Feed::Resources, Some(42), id, NOW).unwrap();
            assert_eq!(Cursor::decode(&value, Feed::Resources, Some(42), NOW).unwrap(), id);
            Cursor::decode(&value, Feed::Resources, Some(43), NOW).unwrap_err();
            Cursor::decode(&value, Feed::Battles, Some(42), NOW).unwrap_err();
            Cursor::decode(&value, Feed::Resources, None, NOW).unwrap_err();
            assert!(matches!(
                Cursor::decode(&value, Feed::Resources, Some(42), NOW + WINDOW_MILLIS + 1),
                Err(ApiError::Gone(_))
            ));
        }
    }

    #[test]
    fn rejects_malformed_unsupported_and_future_cursors() {
        for value in ["", "not a cursor", "e30", &"x".repeat(1025)] {
            Cursor::decode(value, Feed::Battles, None, NOW).unwrap_err();
        }

        for (version, after, issued_at) in
            [(2, None, NOW), (1, Some("bad-id"), NOW), (1, None, NOW + 1)]
        {
            let value = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&serde_json::json!({
                    "version": version, "feed": "battles", "player_id": null,
                    "after": after, "issued_at": issued_at,
                }))
                .unwrap(),
            );
            Cursor::decode(&value, Feed::Battles, None, NOW).unwrap_err();
        }
    }
}
