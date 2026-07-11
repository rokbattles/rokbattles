#![forbid(unsafe_code)]

//! Parses GVE EventMemberLootReport mail.

mod boss;
mod content;
mod metadata;
mod participants;

pub use mail_sdk::{ExtractError, Section};
use mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the GVE member loot report parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(boss::BossExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    const SAMPLES: &[(&str, u64)] = &[
        ("28722408178369207531", 30001),
        ("28725082178369214931", 30002),
        ("28727683178369221531", 30003),
        ("28730088178369228031", 30004),
        ("28732740178369235531", 30005),
    ];

    #[test]
    fn processes_all_five_gve_samples() {
        for (mail_id, boss_id) in SAMPLES {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../../../samples/EventMemberLootReport/Persistent.Mail.{mail_id}.json"
            ));
            let input: Value =
                serde_json::from_str(&fs::read_to_string(path).expect("read sample"))
                    .expect("parse sample");
            let output = process(&input).expect("process sample");
            assert_eq!(output.sections()["metadata"].fields()["mail_id"], json!(mail_id));
            assert_eq!(output.sections()["boss"].fields()["id"], json!(boss_id));

            let participants = output.sections()["participants"].array().expect("participants");
            assert!(participants.len() > 1);
            assert!(participants.iter().all(|participant| {
                participant.get("player_id").is_some()
                    && participant.get("player_name").is_some()
                    && participant.get("avatar_url").is_some()
                    && participant.get("frame_url").is_some()
                    && participant.get("loot").and_then(Value::as_array).is_some()
            }));
        }
    }
}
