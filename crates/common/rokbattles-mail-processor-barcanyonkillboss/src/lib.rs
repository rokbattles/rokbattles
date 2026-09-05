#![forbid(unsafe_code)]

//! Extracts structured sections from decoded BarCanyonKillBoss boss reward mail.
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
//! use rokbattles_mail_processor_barcanyonkillboss::process;
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
