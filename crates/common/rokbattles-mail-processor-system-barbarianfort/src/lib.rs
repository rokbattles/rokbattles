#![forbid(unsafe_code)]

//! Extracts structured sections from decoded System barbarian fort reward mail.
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
//! | `body` | object | Position, target name, subtype, and optional details parsed from localized body text. |
//!
//! The body requires `position`, `targetName`, `subType`, and `subParam`.
//! Recognized localized templates add `body.content` with damage percentage,
//! reward tier, and target level. Missing or unrecognized text simply omits that
//! nested object; it does not discard the structured body fields or rewards.
//! Attachments and each attachment's loot array are required, but may be empty.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_system_barbarianfort::process;
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
mod rewards;
mod templates;

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
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn process_roundtrip_extracts_marauder_encampment_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/System/Persistent.Mail.54530305177357763431.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(processed_json["metadata"]["mail_id"], json!("54530305177357763431"));
        assert_eq!(processed_json["metadata"]["mail_receiver"], json!("player_71738515"));
        assert_eq!(processed_json["body"]["target_name"], json!("Level11"));
        assert_eq!(processed_json["body"]["sub_type"], json!(11));
        assert_eq!(processed_json["body"]["sub_param"], json!(3));
        assert_eq!(
            processed_json["body"]["content"],
            json!({ "percentage": 15.0, "tier": 3, "level": 11 })
        );
        assert_eq!(
            processed_json["body"]["pos"],
            json!({ "x": 7033.7001953125, "y": 1246.9722900390625 })
        );
        assert_eq!(processed_json["rewards"].as_array().map(Vec::len), Some(5));
        assert_eq!(processed_json["rewards"][0], json!({"type": 2, "sub_type": 58, "value": 18}));
    }

    #[test]
    fn process_roundtrip_extracts_french_marauder_content() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/System/Persistent.Mail.62854071172414985228.json");
        let raw = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&raw).expect("parse sample");

        let processed = process(&value).expect("process sample");
        let processed_json = serde_json::to_value(processed).expect("serialize processed");

        assert_eq!(
            processed_json["body"]["content"],
            json!({ "percentage": 0.0, "tier": 1, "level": 1 })
        );
    }
}
