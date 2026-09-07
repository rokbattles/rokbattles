#![forbid(unsafe_code)]

//! Extracts structured sections from decoded Ark of Osiris alliance battle results mail.
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
//! | `alliances` | array | Alliance identities and match totals from `body.kvs.asInfos`. |
//! | `body` | object | Body type, optional parameter, battle type, win flag, and own alliance ID. |
//! | `participants` | array | Individual score rows from `body.kvs.plyRanks`. |
//! | `overview` | object | Alliance scores and optional MVPs from the six `body.kvs.max*` blocks. |
//!
//! All six overview category objects and their `AsScore` values are required.
//! An absent `PlyScore` produces a null MVP; a present null is rejected. Optional
//! alliance identity and team fields become null. Body `type` and `param` accept
//! unsigned integers or numeric strings; the other fields use their declared JSON
//! types. List order and numeric score representations are preserved.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_alliance_aoo_battle_results::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod alliances;
mod body;
mod content;
mod metadata;
mod overview;
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
        Box::new(alliances::AlliancesExtractor::new()),
        Box::new(body::BodyExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
        Box::new(overview::OverviewExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use super::*;

    #[test]
    fn process_extracts_expected_sections() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.102185423177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("alliances"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("participants"));
        assert!(sections.contains_key("overview"));
    }
}
