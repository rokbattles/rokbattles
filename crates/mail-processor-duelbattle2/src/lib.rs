pub mod context;
pub mod error;
mod helpers;
pub mod resolvers;
pub mod structures;

use mail_processor_sdk::ResolverChain;
use serde_json::Value;

use crate::context::MailContext;
use crate::error::ProcessError;
use crate::resolvers::{DetailsResolver, MetadataResolver};
use crate::structures::DuelBattle2Mail;

/// Processes decoded DuelBattle2 mail sections into a structured output.
///
/// The input is expected to be the `sections` array from decoded mail JSON.
pub fn process_sections(sections: &[Value]) -> Result<DuelBattle2Mail, ProcessError> {
    if sections.is_empty() {
        return Err(ProcessError::EmptySections);
    }

    let ctx = MailContext::new(sections);
    let mut output = DuelBattle2Mail::default();

    let chain = ResolverChain::new()
        .with(MetadataResolver::new())
        .with(DetailsResolver::new());
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
    fn process_sections_extracts_details_and_results() {
        let sections = vec![
            json!({
                "id": "mail-3",
                "time": 999,
                "serverId": 15790,
                "body": {
                    "detail": {
                        "AtkPlayer": {
                            "PlayerName": "Attacker",
                            "ServerId": 1804,
                            "Abbr": "AAA",
                            "KillScore": 10,
                            "UnitBadHurt": 2,
                            "UnitHurt": 3,
                            "UnitDead": 4,
                            "UnitReturn": 5,
                            "IsWin": false,
                            "Heroes": {
                                "MainHero": {
                                    "HeroId": 100,
                                    "HeroLevel": 60,
                                    "Star": 6,
                                    "Awaked": true,
                                    "Skills": { "Id": 0, "Level": 5, "SkillId": 1000 },
                                    "Id": 1,
                                    "Level": 2,
                                    "SkillId": 1001
                                },
                                "Id": 2,
                                "Level": 3,
                                "SkillId": 1002
                            },
                            "Id": 3,
                            "Level": 4,
                            "SkillId": 1003
                        },
                        "Id": 4,
                        "Level": 1,
                        "SkillId": 1004
                    }
                }
            }),
            json!({ "Buffs": { "BuffId": 11, "BuffValue": 1 }, "BuffId": 12, "BuffValue": 2 }),
            json!({
                "AssistHero": {
                    "HeroId": 101,
                    "HeroLevel": 50,
                    "Star": 5,
                    "Awaked": false,
                    "Skills": { "Id": 0, "Level": 1, "SkillId": 2000 },
                    "Id": 1,
                    "Level": 2,
                    "SkillId": 2001
                },
                "Id": 2,
                "Level": 3,
                "SkillId": 2002
            }),
            json!({ "Id": 3, "Level": 4, "SkillId": 2003 }),
            json!({ "PosIdx": 11 }),
            json!({
                "DuelTeamId": 111,
                "UnitTotal": 999,
                "PlayerId": 42,
                "LosePower": 888,
                "PlayerAvatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}"
            }),
            json!({
                "DefPlayer": {
                    "PlayerName": "Defender",
                    "ServerId": 1900,
                    "Abbr": "BBB",
                    "KillScore": 20,
                    "UnitBadHurt": 12,
                    "UnitHurt": 13,
                    "UnitDead": 14,
                    "UnitReturn": 15,
                    "IsWin": true,
                    "Heroes": {
                        "MainHero": {
                            "HeroId": 200,
                            "HeroLevel": 40,
                            "Star": 4,
                            "Awaked": false,
                            "Skills": { "Id": 0, "Level": 1, "SkillId": 3000 },
                            "Id": 1,
                            "Level": 2,
                            "SkillId": 3001
                        }
                    },
                    "Id": 2,
                    "Level": 3,
                    "SkillId": 3002
                },
                "Id": 3,
                "Level": 4,
                "SkillId": 3003
            }),
            json!({ "BuffId": 21, "BuffValue": 3.5 }),
            json!({
                "AssistHero": {
                    "HeroId": 201,
                    "HeroLevel": 41,
                    "Star": 4,
                    "Awaked": true,
                    "Skills": { "Id": 0, "Level": 1, "SkillId": 4000 },
                    "Id": 1,
                    "Level": 2,
                    "SkillId": 4001
                },
                "Id": 2,
                "Level": 3,
                "SkillId": 4002
            }),
            json!({ "Id": 3, "Level": 4, "SkillId": 4003 }),
            json!({
                "DuelTeamId": 222,
                "UnitTotal": 555,
                "PlayerId": 84,
                "LosePower": 444,
                "PlayerAvatar": "{\"avatar\":\"http://b\",\"avatarFrame\":\"http://g\"}"
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(output.sender.player_name.as_deref(), Some("Attacker"));
        assert_eq!(output.sender.player_id, Some(42));
        assert_eq!(output.sender.duel_id, Some(111));
        assert_eq!(output.sender.avatar_url.as_deref(), Some("http://a"));
        assert_eq!(output.sender.frame_url.as_deref(), Some("http://f"));
        assert_eq!(output.sender.buffs.len(), 2);

        let primary = output
            .sender
            .commanders
            .primary
            .as_ref()
            .expect("primary commander");
        assert_eq!(primary.id, Some(100));
        assert_eq!(primary.skills.len(), 5);

        let secondary = output
            .sender
            .commanders
            .secondary
            .as_ref()
            .expect("secondary commander");
        assert_eq!(secondary.id, Some(101));
        assert_eq!(secondary.skills.len(), 4);

        assert_eq!(output.results.kill_points, Some(10));
        assert_eq!(output.results.sev_wounded, Some(2));
        assert_eq!(output.results.wounded, Some(3));
        assert_eq!(output.results.dead, Some(4));
        assert_eq!(output.results.heal, Some(5));
        assert_eq!(output.results.units, Some(999));
        assert_eq!(output.results.power, Some(888));
        assert_eq!(output.results.win, Some(false));

        assert_eq!(output.opponent.player_name.as_deref(), Some("Defender"));
        assert_eq!(output.opponent.player_id, Some(84));
        assert_eq!(output.opponent.duel_id, Some(222));

        let opponent_primary = output
            .opponent
            .commanders
            .primary
            .as_ref()
            .expect("opponent primary commander");
        assert_eq!(opponent_primary.id, Some(200));
        assert_eq!(opponent_primary.skills.len(), 4);

        let opponent_secondary = output
            .opponent
            .commanders
            .secondary
            .as_ref()
            .expect("opponent secondary commander");
        assert_eq!(opponent_secondary.id, Some(201));
        assert_eq!(opponent_secondary.skills.len(), 4);

        assert_eq!(output.results.opponent_kill_points, Some(20));
        assert_eq!(output.results.opponent_sev_wounded, Some(12));
        assert_eq!(output.results.opponent_wounded, Some(13));
        assert_eq!(output.results.opponent_dead, Some(14));
        assert_eq!(output.results.opponent_heal, Some(15));
        assert_eq!(output.results.opponent_units, Some(555));
        assert_eq!(output.results.opponent_power, Some(444));
        assert_eq!(output.results.opponent_win, Some(true));
    }

    #[test]
    fn process_sections_rejects_empty_payloads() {
        let err = process_sections(&[]).unwrap_err();

        assert!(matches!(err, ProcessError::EmptySections));
    }
}
