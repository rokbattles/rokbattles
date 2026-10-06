#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Extracts structured sections from decoded KillEliteBarReport elite barbarian reward mail.
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
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_killelitebarreport::process;
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
    use rokbattles_mail_sdk::ProcessError;
    use serde_json::{Value, json};

    use crate::{ExtractError, process};

    fn report() -> Value {
        json!({
            "id": "mail-1",
            "time": 1234,
            "receiver": "player_42",
            "serverId": 55,
            "body": {
                "content": {
                    "npcType": 111,
                    "npcLevel": 1,
                    "pos": { "x": 1.5, "y": 2 },
                    "infos": [
                        {
                            "playerId": 42,
                            "name": "Tester",
                            "damageRate": 55.5,
                            "avatar": "{\"avatar\":\"https://example.com/a.png\",\"avatarFrame\":\"null\"}",
                            "loots": [
                                { "Type": 2, "SubType": 72, "Value": 3 },
                                { "Type": 2, "SubType": 72, "Value": 2 }
                            ]
                        }
                    ]
                }
            }
        })
    }

    #[test]
    fn process_preserves_all_sections_and_duplicate_loot_entries() {
        let output = serde_json::to_value(process(&report()).expect("process report"))
            .expect("serialize report");

        assert_eq!(
            output,
            json!({
                "metadata": {
                    "mail_id": "mail-1",
                    "mail_time": 1234,
                    "mail_receiver": "player_42",
                    "server_id": 55
                },
                "npc": {
                    "type": 111,
                    "level": 1,
                    "location": { "x": 1.5, "y": 2 }
                },
                "participants": [
                    {
                        "player_id": 42,
                        "player_name": "Tester",
                        "damage_rate": 55.5,
                        "avatar_url": "https://example.com/a.png",
                        "frame_url": null,
                        "loot": [
                            { "type": 2, "sub_type": 72, "value": 3 },
                            { "type": 2, "sub_type": 72, "value": 2 }
                        ]
                    }
                ]
            })
        );
    }

    #[test]
    fn process_reads_both_lohar_samples() {
        let cases = [
            (
                include_str!(
                    "../../../../samples/KillEliteBarReport/Persistent.Mail.58081597179130534931.json"
                ),
                "58081597179130534931",
                1791305349582940u64,
                111,
                1,
                json!({ "x": 3816.540771484375, "y": 3958.83203125 }),
                [55.589996337890625, 44.40999984741211],
                [
                    json!([
                        { "type": 2, "sub_type": 1, "value": 3 },
                        { "type": 2, "sub_type": 72, "value": 2 },
                        { "type": 2, "sub_type": 22098, "value": 1 },
                        { "type": 2, "sub_type": 9, "value": 1 },
                        { "type": 2, "sub_type": 94, "value": 1 }
                    ]),
                    json!([
                        { "type": 2, "sub_type": 7, "value": 3 },
                        { "type": 2, "sub_type": 62, "value": 2 },
                        { "type": 2, "sub_type": 10370, "value": 3 }
                    ]),
                ],
            ),
            (
                include_str!(
                    "../../../../samples/KillEliteBarReport/Persistent.Mail.58095236179130567531.json"
                ),
                "58095236179130567531",
                1791305675590863u64,
                112,
                2,
                json!({ "x": 3804.764404296875, "y": 3971.28076171875 }),
                [55.58833312988281, 44.41166687011719],
                [
                    json!([
                        { "type": 2, "sub_type": 7, "value": 5 },
                        { "type": 2, "sub_type": 74, "value": 2 },
                        { "type": 2, "sub_type": 10370, "value": 4 },
                        { "type": 2, "sub_type": 9, "value": 1 },
                        { "type": 2, "sub_type": 75, "value": 2 }
                    ]),
                    json!([
                        { "type": 2, "sub_type": 7, "value": 5 },
                        { "type": 2, "sub_type": 74, "value": 2 },
                        { "type": 2, "sub_type": 10370, "value": 1 },
                        { "type": 2, "sub_type": 9, "value": 1 }
                    ]),
                ],
            ),
        ];

        for (
            sample,
            id,
            time,
            npc_type,
            level,
            location,
            [first_damage, second_damage],
            [first_loot, second_loot],
        ) in cases
        {
            let input = serde_json::from_str(sample).expect("parse sample");

            let output = serde_json::to_value(process(&input).expect("process sample"))
                .expect("serialize report");

            assert_eq!(
                output,
                json!({
                    "metadata": {
                        "mail_id": id,
                        "mail_time": time,
                        "mail_receiver": "player_71738515",
                        "server_id": 1804
                    },
                    "npc": {
                        "type": npc_type,
                        "level": level,
                        "location": location
                    },
                    "participants": [
                        {
                            "player_id": 71738515,
                            "player_name": "Grigvar",
                            "damage_rate": first_damage,
                            "avatar_url": "http://imimg.lilithcdn.com/roc/img_player_head_festive_1022.png",
                            "frame_url": null,
                            "loot": first_loot
                        },
                        {
                            "player_id": 109277746,
                            "player_name": "Fayst cav",
                            "damage_rate": second_damage,
                            "avatar_url": "https://plat-fau-global.lilithgame.com/p/astc/IM/10043/0/88504567/2026-07-03/91D0A53A54563BAD688D6A8E5B22B5E7_250x250.jpg",
                            "frame_url": "http://imimg.lilithcdn.com/roc/img_AvatarFrame_239.png",
                            "loot": second_loot
                        }
                    ]
                }),
                "sample {id}"
            );
        }
    }

    #[test]
    fn process_normalizes_supported_avatar_shapes() {
        for (avatar, expected_url, expected_frame) in [
            (Value::Null, Value::Null, Value::Null),
            (json!("null"), Value::Null, Value::Null),
            (json!({}), Value::Null, Value::Null),
            (json!("https://example.com/a.png"), json!("https://example.com/a.png"), Value::Null),
            (
                json!({ "avatar": "null", "avatarFrame": "frame.png" }),
                Value::Null,
                json!("frame.png"),
            ),
        ] {
            let mut input = report();
            input["body"]["content"]["infos"][0]["avatar"] = avatar;

            let output = serde_json::to_value(process(&input).expect("process avatar"))
                .expect("serialize report");

            assert_eq!(output["participants"][0]["avatar_url"], expected_url);
            assert_eq!(output["participants"][0]["frame_url"], expected_frame);
        }
    }

    #[test]
    fn process_accepts_empty_participants_and_loot() {
        for (pointer, output_pointer) in [
            ("/body/content/infos", "/participants"),
            ("/body/content/infos/0/loots", "/participants/0/loot"),
        ] {
            let mut input = report();
            *input.pointer_mut(pointer).expect("input field") = json!([]);

            let output = serde_json::to_value(process(&input).expect("process empty array"))
                .expect("serialize report");

            assert_eq!(output.pointer(output_pointer), Some(&json!([])));
        }
    }

    #[test]
    fn process_rejects_missing_fields_with_section_and_field() {
        for (parent, field, expected_section) in [
            ("", "id", "metadata"),
            ("", "time", "metadata"),
            ("", "receiver", "metadata"),
            ("", "serverId", "metadata"),
            ("/body", "content", "npc"),
            ("/body/content", "npcType", "npc"),
            ("/body/content", "npcLevel", "npc"),
            ("/body/content", "pos", "npc"),
            ("/body/content/pos", "x", "npc"),
            ("/body/content/pos", "y", "npc"),
            ("/body/content", "infos", "participants"),
            ("/body/content/infos/0", "playerId", "participants"),
            ("/body/content/infos/0", "name", "participants"),
            ("/body/content/infos/0", "damageRate", "participants"),
            ("/body/content/infos/0", "avatar", "participants"),
            ("/body/content/infos/0", "loots", "participants"),
            ("/body/content/infos/0/loots/0", "Type", "participants"),
            ("/body/content/infos/0/loots/0", "SubType", "participants"),
            ("/body/content/infos/0/loots/0", "Value", "participants"),
        ] {
            let mut input = report();
            input
                .pointer_mut(parent)
                .expect("parent")
                .as_object_mut()
                .expect("object")
                .remove(field);

            let error = process(&input).expect_err("missing required field");

            let ProcessError::ExtractorFailed { section, source } = error else {
                panic!("unexpected error: {error}");
            };

            assert_eq!((section, source), (expected_section, ExtractError::MissingField { field }));
        }
    }

    #[test]
    fn process_rejects_invalid_field_types() {
        for (pointer, value, expected_section, field, expected) in [
            ("/time", json!(-1), "metadata", "time", "unsigned integer"),
            ("/body", json!([]), "npc", "body", "object"),
            ("/body/content/npcType", json!("111"), "npc", "npcType", "unsigned integer"),
            ("/body/content/npcLevel", json!(-1), "npc", "npcLevel", "unsigned integer"),
            ("/body/content/pos/x", json!("1.5"), "npc", "x", "number"),
            ("/body/content/infos", json!({}), "participants", "infos", "array"),
            ("/body/content/infos/0", Value::Null, "participants", "infos", "object"),
            (
                "/body/content/infos/0/damageRate",
                json!("55.5"),
                "participants",
                "damageRate",
                "number",
            ),
            (
                "/body/content/infos/0/avatar",
                json!(1),
                "participants",
                "avatar",
                "string or object",
            ),
            ("/body/content/infos/0/loots", Value::Null, "participants", "loots", "array"),
            ("/body/content/infos/0/loots/0", json!(1), "participants", "loots", "object"),
            (
                "/body/content/infos/0/loots/0/Value",
                json!(-1),
                "participants",
                "Value",
                "unsigned integer",
            ),
        ] {
            let mut input = report();
            *input.pointer_mut(pointer).expect("input field") = value;

            let error = process(&input).expect_err("invalid field type");

            let ProcessError::ExtractorFailed { section, source } = error else {
                panic!("unexpected error: {error}");
            };

            assert_eq!(
                (section, source),
                (expected_section, ExtractError::InvalidFieldType { field, expected })
            );
        }
    }
}
