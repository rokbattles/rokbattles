pub mod context;
pub mod error;
pub mod resolvers;
pub mod structures;

use mail_processor_sdk::ResolverChain;
use serde_json::Value;

use crate::context::MailContext;
use crate::error::ProcessError;
use crate::resolvers::{DataSummaryResolver, MetadataResolver};
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

    let chain = ResolverChain::new()
        .with(MetadataResolver::new())
        .with(DataSummaryResolver::new());
    chain.apply(&ctx, &mut output)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::process_sections;
    use crate::error::ProcessError;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::Path;

    fn load_sample_sections(name: &str) -> Vec<Value> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("../../samples/Battle").join(name);
        let payload =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"));
        let root: Value =
            serde_json::from_str(&payload).unwrap_or_else(|err| panic!("parse {path:?}: {err}"));
        root.get("sections")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("missing sections in {path:?}"))
            .to_vec()
    }

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

    #[test]
    fn process_sections_leaves_data_summary_empty_without_overview() {
        let sections = vec![json!({
            "id": "mail-14",
            "time": 1111,
            "serverId": 1804
        })];

        let output = process_sections(&sections).expect("process mail");

        assert!(output.data_summary.is_none());
    }

    #[test]
    fn process_sections_loads_data_summary_from_same_section() {
        let sections = load_sample_sections("10224136175529255431.json");

        let output = process_sections(&sections).expect("process mail");
        let summary = output.data_summary.expect("data summary");

        assert_eq!(summary.sender_kill_points, Some(307090));
        assert_eq!(summary.sender_severely_wounded, Some(14528));
        assert_eq!(summary.sender_slightly_wounded, Some(85754));
        assert_eq!(summary.sender_troop_units, Some(230000));
        assert_eq!(summary.sender_remaining, Some(129718));
        assert_eq!(summary.sender_dead, Some(0));
        assert_eq!(summary.opponent_kill_points, Some(290560));
        assert_eq!(summary.opponent_severely_wounded, Some(30709));
        assert_eq!(summary.opponent_slightly_wounded, Some(166508));
        assert_eq!(summary.opponent_troop_units, Some(418491));
        assert_eq!(summary.opponent_remaining, Some(17659));
        assert_eq!(summary.opponent_dead, Some(0));
    }

    #[test]
    fn process_sections_loads_data_summary_from_separate_sections() {
        let sections = load_sample_sections("1764512944407776.json");

        let output = process_sections(&sections).expect("process mail");
        let summary = output.data_summary.expect("data summary");

        assert_eq!(summary.sender_kill_points, Some(16991393));
        assert_eq!(summary.sender_severely_wounded, Some(272818));
        assert_eq!(summary.sender_slightly_wounded, Some(1400260));
        assert_eq!(summary.sender_troop_units, Some(2730000));
        assert_eq!(summary.sender_remaining, Some(1056922));
        assert_eq!(summary.sender_dead, Some(0));
        assert_eq!(summary.opponent_kill_points, Some(5212360));
        assert_eq!(summary.opponent_severely_wounded, Some(859533));
        assert_eq!(summary.opponent_slightly_wounded, Some(3279249));
        assert_eq!(summary.opponent_troop_units, Some(6804444));
        assert_eq!(summary.opponent_remaining, Some(116032));
        assert_eq!(summary.opponent_dead, Some(0));
    }
}
