use mongodb::{
    Collection,
    bson::{Document, doc},
    options::FindOneOptions,
};
use rokbattles_bson::bson_to_i64_exact;

use crate::{db::exclude_test_reports_filter, error::ApiError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GovernorSnapshot {
    pub app_uid: Option<i64>,
    pub governor_name: Option<String>,
    pub governor_avatar: Option<String>,
}

/// Returns the latest eligible battle report's sender details for the governor.
///
/// Only reports with a positive sender Lilith ID are eligible for this lookup.
pub async fn find_latest_sender_snapshot(
    battle_reports: &Collection<Document>,
    governor_id: i64,
) -> Result<Option<GovernorSnapshot>, ApiError> {
    let latest_mail = battle_reports
        .find_one(doc! {
            "$and": [
                exclude_test_reports_filter(),
                { "sender.player_id": governor_id, "sender.app_uid": { "$gt": 0 } },
            ]
        })
        .with_options(
            FindOneOptions::builder()
                .sort(doc! { "metadata.mail_time": -1, "_id": -1 })
                .projection(doc! {
                    "sender.app_uid": 1,
                    "sender.player_name": 1,
                    "sender.avatar_url": 1,
                })
                .build(),
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(latest_mail.as_ref().and_then(extract_sender_snapshot))
}

fn extract_sender_snapshot(mail: &Document) -> Option<GovernorSnapshot> {
    let sender = mail.get_document("sender").ok()?;
    Some(snapshot_from_participant(sender))
}

fn snapshot_from_participant(participant: &Document) -> GovernorSnapshot {
    GovernorSnapshot {
        app_uid: participant.get("app_uid").and_then(bson_to_i64_exact).filter(|id| *id > 0),
        governor_name: participant.get_str("player_name").ok().map(ToString::to_string),
        governor_avatar: participant.get_str("avatar_url").ok().map(ToString::to_string),
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::*;

    #[test]
    fn extracts_sender_snapshot_when_sender_is_present() {
        let mail = doc! {
            "sender": {
                "player_id": 1001,
                "app_uid": 5_i64,
                "player_name": "Sender",
                "avatar_url": "sender.png",
            },
        };

        let snapshot = extract_sender_snapshot(&mail).expect("sender snapshot");
        assert_eq!(
            snapshot,
            GovernorSnapshot {
                app_uid: Some(5),
                governor_name: Some("Sender".to_string()),
                governor_avatar: Some("sender.png".to_string()),
            }
        );
    }

    #[test]
    fn returns_none_when_sender_is_missing() {
        let mail = doc! {
            "metadata": {
                "mail_time": 1
            }
        };

        assert_eq!(extract_sender_snapshot(&mail), None);
    }
}
