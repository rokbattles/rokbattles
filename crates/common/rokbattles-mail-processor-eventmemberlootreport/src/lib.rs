#![forbid(unsafe_code)]

//! Extracts structured sections from decoded GVE EventMemberLootReport mail.
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
//! | `boss` | object | A stable boss ID inferred from `body.content.subTitle`. |
//! | `participants` | array | Player identity, avatars, and loot from `body.content.infos`. |
//!
//! The boss extractor matches known localized names as case-sensitive substrings
//! of the subtitle. An unknown subtitle fails extraction. Participants and their
//! loot retain their input order. The registry checks `EventName == "GVE"` when
//! routing mail; this processor does not repeat that category check.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_eventmemberlootreport::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod boss;
mod content;
mod metadata;
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
                "../../../samples/EventMemberLootReport/Persistent.Mail.{mail_id}.json"
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
