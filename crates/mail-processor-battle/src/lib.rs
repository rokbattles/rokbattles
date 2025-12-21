pub mod context;
pub mod error;
pub mod resolvers;
pub mod structures;

use mail_processor_sdk::ResolverChain;
use serde_json::Value;

use crate::context::MailContext;
use crate::error::ProcessError;
use crate::resolvers::MetadataResolver;
use crate::structures::BattleMail;

/// Processes decoded Battle mail sections into a structured output.
///
/// The input is expected to be the `sections` array from decoded mail JSON.
pub fn process_sections(sections: &[Value]) -> Result<BattleMail, ProcessError> {
    if sections.is_empty() {
        return Err(ProcessError::EmptySections);
    }

    let ctx = MailContext::new(sections);
    let mut output = BattleMail::default();

    let chain = ResolverChain::new().with(MetadataResolver::new());
    chain.apply(&ctx, &mut output)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::process_sections;
    use crate::error::ProcessError;
    use serde_json::json;

    #[test]
    fn process_sections_populates_header_metadata() {
        let sections = vec![json!({
            "id": "mail-1",
            "time": 123,
            "serverId": 1804
        })];

        let output = process_sections(&sections).expect("process mail");
        let meta = output.metadata;

        assert_eq!(meta.email_id.as_deref(), Some("mail-1"));
        assert_eq!(meta.email_time, Some(123));
        assert_eq!(meta.server_id, Some(1804));
    }

    #[test]
    fn process_sections_scans_for_receiver() {
        let sections = vec![
            json!({
                "id": "mail-2",
                "time": 456,
            }),
            json!({
                "receiver": "player_123"
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(
            output.metadata.email_receiver.as_deref(),
            Some("player_123")
        );
    }

    #[test]
    fn process_sections_parses_numeric_and_string_values() {
        let sections = vec![json!({
            "id": 987654,
            "time": "321",
            "serverId": "42"
        })];

        let output = process_sections(&sections).expect("process mail");
        let meta = output.metadata;

        assert_eq!(meta.email_id.as_deref(), Some("987654"));
        assert_eq!(meta.email_time, Some(321));
        assert_eq!(meta.server_id, Some(42));
    }

    #[test]
    fn process_sections_rejects_empty_payloads() {
        let err = process_sections(&[]).unwrap_err();

        assert!(matches!(err, ProcessError::EmptySections));
    }

    #[test]
    fn process_sections_sets_rokb_email_type_ark() {
        let sections = vec![
            json!({
                "id": "mail-3",
                "time": 789,
                "serverId": 1804
            }),
            json!({
                "Role": "dungeon"
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_email_type.as_deref(), Some("ark"));
    }

    #[test]
    fn process_sections_sets_rokb_email_type_kvk() {
        let sections = vec![
            json!({
                "id": "mail-4",
                "time": 101,
                "serverId": 1804
            }),
            json!({
                "Role": "gsmp",
                "isConquerSeason": true
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_email_type.as_deref(), Some("kvk"));
    }

    #[test]
    fn process_sections_sets_rokb_email_type_kvk_when_server_mismatch() {
        let sections = vec![
            json!({
                "id": "mail-6",
                "time": 303,
                "serverId": 1550
            }),
            json!({
                "COSId": 1804
            }),
            json!({
                "Role": "gsmp"
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_email_type.as_deref(), Some("kvk"));
    }

    #[test]
    fn process_sections_prefers_conquer_season_over_server_match() {
        let sections = vec![
            json!({
                "id": "mail-7",
                "time": 404,
                "serverId": 1804
            }),
            json!({
                "COSId": 1804
            }),
            json!({
                "Role": "gsmp",
                "isConquerSeason": true
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_email_type.as_deref(), Some("kvk"));
    }

    #[test]
    fn process_sections_sets_rokb_email_type_home() {
        let sections = vec![
            json!({
                "id": "mail-5",
                "time": 202,
                "serverId": 1804
            }),
            json!({
                "COSId": 1804
            }),
            json!({
                "Role": "gs"
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_email_type.as_deref(), Some("home"));
    }

    #[test]
    fn process_sections_sets_rokb_email_type_home_from_body_content_cos_id() {
        let sections = vec![json!({
            "id": "mail-8",
            "time": 505,
            "serverId": 1804,
            "body": {
                "Role": "gsmp",
                "content": {
                    "COSId": 1804
                }
            }
        })];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_email_type.as_deref(), Some("home"));
    }

    #[test]
    fn process_sections_sets_rokb_battle_type_open_field_when_sender_has_no_flags() {
        let sections = vec![
            json!({
                "id": "mail-9",
                "time": 606,
                "serverId": 1804
            }),
            json!({
                "PName": "Sender",
                "COSId": 1804,
                "IsRally": 0
            }),
            json!({
                "PName": "Opponent",
                "COSId": 1900
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(
            output.metadata.rokb_battle_type.as_deref(),
            Some("open_field")
        );
    }

    #[test]
    fn process_sections_sets_rokb_battle_type_rally_from_sender_sections() {
        let sections = vec![
            json!({
                "id": "mail-10",
                "time": 707,
                "serverId": 1804
            }),
            json!({
                "PName": "Sender",
                "COSId": 1804
            }),
            json!({
                "PName": "RallyMember",
                "COSId": 1804,
                "IsRally": true
            }),
            json!({
                "PName": "Opponent",
                "COSId": 2000
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.metadata.rokb_battle_type.as_deref(), Some("rally"));
    }

    #[test]
    fn process_sections_sets_rokb_battle_type_garrison_from_sender_abt() {
        let sections = vec![
            json!({
                "id": "mail-11",
                "time": 808,
                "serverId": 1804
            }),
            json!({
                "PName": "Sender",
                "COSId": 1804,
                "AbT": 3
            }),
            json!({
                "PName": "RallyMember",
                "COSId": 1804,
                "IsRally": true
            }),
            json!({
                "PName": "Opponent",
                "COSId": 2200
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(
            output.metadata.rokb_battle_type.as_deref(),
            Some("garrison")
        );
    }

    #[test]
    fn process_sections_ignores_opponent_battle_flags() {
        let sections = vec![
            json!({
                "id": "mail-12",
                "time": 909,
                "serverId": 1804
            }),
            json!({
                "PName": "Sender",
                "COSId": 1804
            }),
            json!({
                "PName": "Opponent",
                "COSId": 2500,
                "AbT": 7
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(
            output.metadata.rokb_battle_type.as_deref(),
            Some("open_field")
        );
    }

    #[test]
    fn process_sections_sets_rokb_battle_type_from_body_content_sender() {
        let sections = vec![json!({
            "id": "mail-13",
            "time": 1010,
            "serverId": 1804,
            "body": {
                "content": {
                    "PName": "Grigvar",
                    "COSId": 1804,
                    "CT": 1
                }
            }
        })];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(
            output.metadata.rokb_battle_type.as_deref(),
            Some("open_field")
        );
    }
}
