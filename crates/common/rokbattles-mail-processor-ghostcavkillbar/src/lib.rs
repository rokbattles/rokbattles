#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Extracts structured sections from decoded GhostCavKillBar event bandit reward mail.
//!
//! Pass the decoded root object to [`process`]. The caller selects the mail
//! category; this crate does not decode binary files or validate the root `type`
//! label. Field names are case-sensitive.
//!
//! # Sections
//!
//! | Section | Shape | Contents |
//! | --- | --- | --- |
//! | `metadata` | object | Root `id`, `time`, `receiver`, and `serverId` under the standard SDK field names. |
//! | `npc` | object | NPC type, level, and location from `body.content`. |
//! | `participants` | array | Player identity, damage share, avatars, and loot from `body.content.infos`. |
//!
//! Participant and loot arrays retain their input order. NPC IDs are copied
//! without restricting them to a known boss list. Each participant requires an
//! `avatar` field, which may be a URL string, a JSON-encoded object, an object, or
//! null. Missing avatar/frame members and literal `"null"` values become JSON null.
//!
//! The client derives `eliteBarName` from NPC configuration and level. It also
//! assigns `idx`, supplies display defaults, and sorts participants for display.
//! Processing uses the stable numeric NPC fields and preserves the supplied row
//! order; it does not infer identity from localized names or retain display indices.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_ghostcavkillbar::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod content;
mod metadata;
mod npc;
mod participants;

pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Extracts the sections described in the [crate documentation](crate#sections).
///
/// Borrows `input` and returns owned section data. The SDK runs independent
/// section extractors on scoped threads; no partial output is returned on error.
///
/// # Errors
///
/// Returns [`ProcessError::ExtractorFailed`] with the section name and original
/// [`ExtractError`] when a required value is absent or invalid. Optional fields
/// use the format-specific defaults described above; other invalid values fail
/// extraction. Worker and section-name failures follow the SDK's
/// [`Processor::process`] behavior.
///
/// # Panics
///
/// Has the thread-spawning and panic-propagation behavior of [`Processor::process`].
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(npc::NpcExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn sample() -> Value {
        serde_json::from_str(include_str!(
            "../../../../samples/GhostCavKillBar/Persistent.Mail.13411379179417835931.json"
        ))
        .expect("parse GhostCavKillBar sample")
    }

    fn output(input: &Value) -> Value {
        serde_json::to_value(process(input).expect("process report")).expect("serialize report")
    }

    #[test]
    fn processes_saved_bandit_reward_report() {
        assert_eq!(
            output(&sample()),
            json!({
                "metadata": {
                    "mail_id": "13411379179417835931",
                    "mail_time": 1794178359106490u64,
                    "mail_receiver": "player_71738515",
                    "server_id": 16153
                },
                "npc": {
                    "type": 70005,
                    "level": 1,
                    "location": {"x": 4476.6796875, "y": 4569.76708984375}
                },
                "participants": [{
                    "player_id": 71738515,
                    "player_name": "Grigvar",
                    "damage_rate": 100,
                    "avatar_url": "http://imimg.lilithcdn.com/roc/img_player_head_festive_1022.png",
                    "frame_url": null,
                    "loot": [
                        {"type": 2, "sub_type": 9, "value": 1},
                        {"type": 2, "sub_type": 75, "value": 1},
                        {"type": 2, "sub_type": 47, "value": 1},
                        {"type": 2, "sub_type": 7001, "value": 15},
                        {"type": 2, "sub_type": 8450, "value": 2}
                    ]
                }]
            })
        );
    }

    #[test]
    fn processing_does_not_depend_on_client_injected_display_fields() {
        let mut input = sample();
        let expected = output(&input);
        let content = input["body"]["content"].as_object_mut().expect("content object");
        content.remove("eliteBarName");
        let participant = content["infos"][0].as_object_mut().expect("participant object");
        participant.remove("idx");
        participant.remove("order");
        assert_eq!(output(&input), expected);
    }

    #[test]
    fn preserves_participant_order_damage_and_duplicate_rewards() {
        let mut input = sample();
        let mut second = input["body"]["content"]["infos"][0].clone();
        second["playerId"] = json!(42);
        second["damageRate"] = json!(12.5);
        second["avatar"] = json!({"avatar": "other.png", "avatarFrame": "frame.png"});
        second["loots"] = json!([
            {"Type": 2, "SubType": 9, "Value": 2},
            {"Type": 2, "SubType": 9, "Value": 1}
        ]);
        input["body"]["content"]["infos"].as_array_mut().expect("participants").push(second);
        let result = output(&input);
        assert_eq!(result["participants"][0]["player_id"], json!(71738515));
        assert_eq!(
            result["participants"][1],
            json!({
                "player_id": 42,
                "player_name": "Grigvar",
                "damage_rate": 12.5,
                "avatar_url": "other.png",
                "frame_url": "frame.png",
                "loot": [
                    {"type": 2, "sub_type": 9, "value": 2},
                    {"type": 2, "sub_type": 9, "value": 1}
                ]
            })
        );
    }

    #[test]
    fn accepts_empty_participants_and_empty_loot() {
        for (input_path, output_path) in [
            ("/body/content/infos", "/participants"),
            ("/body/content/infos/0/loots", "/participants/0/loot"),
        ] {
            let mut input = sample();
            *input.pointer_mut(input_path).expect("input array") = json!([]);
            assert_eq!(output(&input).pointer(output_path), Some(&json!([])));
        }
    }

    #[test]
    fn rejects_missing_or_invalid_report_fields() {
        for (parent, field, section) in [
            ("", "id", "metadata"),
            ("/body/content", "npcType", "npc"),
            ("/body/content/pos", "x", "npc"),
            ("/body/content", "infos", "participants"),
            ("/body/content/infos/0", "playerId", "participants"),
            ("/body/content/infos/0", "damageRate", "participants"),
            ("/body/content/infos/0/loots/0", "Value", "participants"),
        ] {
            for invalid in [false, true] {
                let mut input = sample();
                let object =
                    input.pointer_mut(parent).expect("parent").as_object_mut().expect("object");
                if invalid {
                    object.insert(field.to_string(), Value::Null);
                } else {
                    object.remove(field);
                }
                let ProcessError::ExtractorFailed { section: actual_section, source } =
                    process(&input).expect_err("invalid report")
                else {
                    panic!("expected field extraction error");
                };
                assert_eq!(actual_section, section);
                if invalid {
                    assert!(
                        matches!(source, ExtractError::InvalidFieldType {field: actual, ..} if actual == field)
                    );
                } else {
                    assert_eq!(source, ExtractError::MissingField { field });
                }
            }
        }
    }
}
