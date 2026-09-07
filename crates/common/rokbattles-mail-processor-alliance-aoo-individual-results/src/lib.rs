#![forbid(unsafe_code)]

//! Extracts structured sections from decoded Ark of Osiris individual results mail.
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
//! | `rewards` | array | Flattened `attachments[].loot[]` entries. |
//! | `body` | object | Body type, optional parameter, win flag, and optional team index. |
//! | `overview` | object | Rank, optional ranked player, and optional wild-battle totals. |
//! | `pairings` | array | Commander pairing counters from `FightReport.Stat.HerosStat`. |
//! | `results` | object | Individual match counters from `body.kvs.FightReport`. |
//!
//! `body.kvs` and `TotalScoreRank.Rank` are required. Missing or null `FightReport`
//! produces null result fields, null overview totals, and an empty pairing array.
//! If the report exists, its result counters are required except `HealingScore`,
//! which defaults to zero. An empty array is accepted as an absent `Stat` table.
//! Body `type` and `param` also accept numeric strings. Reward arrays are required.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_alliance_aoo_individual_results::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod body;
mod content;
mod metadata;
mod overview;
mod pairings;
mod results;
mod rewards;

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
        Box::new(rewards::RewardsExtractor::new()),
        Box::new(body::BodyExtractor::new()),
        Box::new(overview::OverviewExtractor::new()),
        Box::new(pairings::PairingsExtractor::new()),
        Box::new(results::ResultsExtractor::new()),
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
            .join("../../../samples/Alliance/Persistent.Mail.102185429177177256731.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
    }

    #[test]
    fn process_extracts_sparse_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.6890312417293500508.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
    }

    #[test]
    fn process_extracts_empty_stat_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.91536773174395176822.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
    }

    #[test]
    fn process_extracts_sample_without_healing_score() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Alliance/Persistent.Mail.71266849169063933424.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let sections = processed.sections();
        assert!(sections.contains_key("metadata"));
        assert!(sections.contains_key("rewards"));
        assert!(sections.contains_key("body"));
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("pairings"));
        assert!(sections.contains_key("results"));
        assert_eq!(sections["results"].fields()["healing_score"], serde_json::json!(0));
    }
}
