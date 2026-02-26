use mongodb::Collection;
use mongodb::bson::{Document, doc};
use mongodb::options::FindOneOptions;

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernorSnapshot {
    pub governor_name: Option<String>,
    pub governor_avatar: Option<String>,
}

/// Find the latest sender snapshot for a governor in battle mails.
pub async fn find_latest_sender_snapshot(
    battle_reports: &Collection<Document>,
    governor_id: i64,
) -> Result<Option<GovernorSnapshot>, ApiError> {
    let latest_mail = battle_reports
        .find_one(doc! { "sender.player_id": governor_id })
        .with_options(
            FindOneOptions::builder()
                .sort(doc! { "metadata.mail_time": -1 })
                .projection(doc! {
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
        governor_name: participant
            .get_str("player_name")
            .ok()
            .map(ToString::to_string),
        governor_avatar: participant
            .get_str("avatar_url")
            .ok()
            .map(ToString::to_string),
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
                "player_name": "Sender",
                "avatar_url": "sender.png",
            },
        };

        let snapshot = extract_sender_snapshot(&mail).expect("sender snapshot");
        assert_eq!(
            snapshot,
            GovernorSnapshot {
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
