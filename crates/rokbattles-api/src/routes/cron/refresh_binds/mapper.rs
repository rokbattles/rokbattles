use mongodb::bson::{Bson, Document};

use super::types::GovernorSnapshot;

/// Read governor name/avatar from the mail sender.
pub(super) fn extract_sender_snapshot(mail: &Document) -> Option<GovernorSnapshot> {
    let sender = mail.get_document("sender").ok()?;
    Some(snapshot_from_participant(sender))
}

/// Convert BSON number types we expect into i64.
pub(super) fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) => {
            if value.is_finite() {
                Some(*value as i64)
            } else {
                None
            }
        }
        _ => None,
    }
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
    fn extracts_sender_snapshot() {
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

    #[test]
    fn converts_bson_number_to_i64() {
        assert_eq!(bson_to_i64(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_to_i64(&Bson::Double(56.8)), Some(56));
        assert_eq!(bson_to_i64(&Bson::Null), None);
    }
}
