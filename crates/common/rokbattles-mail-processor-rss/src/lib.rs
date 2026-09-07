#![forbid(unsafe_code)]

//! Extracts structured sections from decoded resource gathering (Rss) mail.
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
//! | `rss` | object | Resource type, amount, bonus, time, level, position, and crystal gain from `body.content`. |
//!
//! All resource fields are required numbers except `ResCollectCrystal`, which
//! becomes zero when missing or null. Numeric values and timestamps keep their
//! input representation and units; `Pos.X` and `Pos.Y` become `pos.x` and `pos.y`.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_rss::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod content;
mod metadata;
mod rss;

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
        Box::new(rss::RssExtractor::new()),
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
            .join("../../../samples/Rss/Persistent.Mail.113157979177212756131.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rss"));
    }

    #[test]
    fn process_extracts_sample_without_crystals_gain() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Rss/Persistent.Mail.118801516499340535.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rss"));
        assert_eq!(sections["rss"].fields()["crystals_gain"], serde_json::json!(0));
    }
}
