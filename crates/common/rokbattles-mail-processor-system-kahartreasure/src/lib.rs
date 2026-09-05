#![forbid(unsafe_code)]

//! Extracts structured sections from decoded System Kahar treasure reward mail.
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
//! | `loot` | array | All `attachments[].loot[]` entries, flattened in attachment order. |
//!
//! Attachments and each attachment's `loot` array are required, but may be empty.
//! Each loot entry requires unsigned integer `Type`, `SubType`, and `Value` fields,
//! renamed to `type`, `sub_type`, and `value`. Duplicate entries remain separate;
//! amounts are not combined.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_system_kahartreasure::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod loot;
mod metadata;

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
        Box::new(loot::LootExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn process_roundtrip_extracts_kahar_treasure_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/System/Persistent.Mail.22165348178347040031.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(processed_json["metadata"]["mail_id"], json!("22165348178347040031"));
        assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(processed_json["metadata"]["server_id"], json!(16012));
        assert_eq!(processed_json["metadata"]["mail_time"], json!(1783470400647228u64));
        assert_eq!(processed_json["loot"].as_array().map(Vec::len), Some(5));
        assert_eq!(processed_json["loot"][0], json!({"type": 1, "sub_type": 9, "value": 45000}));
    }
}
