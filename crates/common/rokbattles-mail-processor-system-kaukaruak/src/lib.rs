#![forbid(unsafe_code)]

//! Processes System reward mails for the Trial of Kau Karuak, separate from
//! Karuak Ceremony alliance boss rewards. The registry selects subtype 149,
//! subparam 1; this processor expects an already-decoded, classified mail.
//!
//! Output contains standard `metadata`, attachment `rewards` in their original
//! order, and `body.sub_type` / `body.sub_param`. Official localized subtitles
//! add `body.content.level` and `body.content.difficulty` (easy, normal, hard,
//! nightmare, or hell). Both legacy 30-level and newer 15-level trials
//! use the same template; no event version is inferred from a level alone.
//! Missing or unrecognized subtitle text omits `body.content` without losing
//! rewards. Attachments and loot arrays are required but may be empty.

mod body;
mod metadata;
mod rewards;
mod templates;

pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Extracts metadata, rewards, and localized trial details.
///
/// # Errors
/// Returns [`ProcessError`] when a required structured field is absent or invalid.
///
/// # Panics
/// Has the thread-spawning and panic-propagation behavior of [`Processor::process`].
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(rewards::RewardsExtractor::new()),
        Box::new(body::BodyExtractor),
    ])
    .process(input)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::json;

    use super::*;

    fn sample(id: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../../samples/System/Persistent.Mail.{id}.json"));

        serde_json::from_slice(&fs::read(path).expect("read sample")).expect("parse sample")
    }

    #[test]
    fn processes_legacy_and_current_trial_samples() {
        for (id, level, difficulty) in [
            ("7330142174194043231", 1, "hard"),
            ("9229521174198158831", 16, "hard"),
            ("9232167174198164831", 17, "hard"),
            ("54634710176791113330", 1, "normal"),
            ("54645627176791161930", 15, "normal"),
            ("49117718179110236622", 1, "nightmare"),
            ("49147831179110293022", 15, "nightmare"),
        ] {
            let input = sample(id);
            let output = serde_json::to_value(process(&input).expect("process sample")).unwrap();

            assert_eq!(
                output["metadata"],
                json!({
                    "mail_id": id,
                    "mail_time": input["time"],
                    "mail_receiver": input["receiver"],
                    "server_id": input["serverId"],
                })
            );
            assert_eq!(
                output["body"],
                json!({
                    "sub_type": 149,
                    "sub_param": 1,
                    "content": {"level": level, "difficulty": difficulty},
                })
            );

            let expected: Vec<_> = input["attachments"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|attachment| attachment["loot"].as_array().unwrap())
                .map(|entry| {
                    json!({
                        "type": entry["Type"],
                        "sub_type": entry["SubType"],
                        "value": entry["Value"],
                    })
                })
                .collect();

            assert_eq!(output["rewards"], json!(expected));
        }
    }

    #[test]
    fn preserves_rewards_without_recognized_localized_text() {
        let mut input = sample("49147831179110293022");
        let expected = process(&input).unwrap().sections()["rewards"].clone();

        for subtitle in [Value::Null, json!(42), json!(""), json!("unknown translation")] {
            input["body"]["subTitle"] = subtitle;
            let output = process(&input).unwrap();

            assert_eq!(output.sections()["rewards"], expected);
            assert!(!output.sections()["body"].fields().contains_key("content"));
        }

        input["body"].as_object_mut().unwrap().remove("subTitle");
        assert_eq!(process(&input).unwrap().sections()["rewards"], expected);
    }

    #[test]
    fn preserves_duplicate_rewards_and_attachment_order() {
        let mut input = sample("49147831179110293022");
        input["attachments"] = json!([
            {
                "loot": [
                    {"Type": 2, "SubType": 234, "Value": 100},
                    {"Type": 2, "SubType": 234, "Value": 200}
                ]
            },
            {"loot": []},
            {"loot": [{"Type": 2, "SubType": 234, "Value": 300}]}
        ]);

        let output = serde_json::to_value(process(&input).unwrap()).unwrap();

        assert_eq!(
            output["rewards"],
            json!([
                {"type": 2, "sub_type": 234, "value": 100},
                {"type": 2, "sub_type": 234, "value": 200},
                {"type": 2, "sub_type": 234, "value": 300}
            ])
        );
    }

    #[test]
    fn rejects_missing_or_malformed_reward_data() {
        for attachments in [
            json!(null),
            json!([null]),
            json!([{}]),
            json!([{"loot": null}]),
            json!([{"loot": [null]}]),
            json!([{"loot": [{"Type": 2, "SubType": 234, "Value": -1}]}]),
        ] {
            let mut input = sample("49147831179110293022");
            input["attachments"] = attachments;

            assert!(matches!(process(&input), Err(ProcessError::ExtractorFailed { .. })));
        }
    }
}
