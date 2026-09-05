#![forbid(unsafe_code)]

//! Extracts structured sections from decoded Battle mail.
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
//! | `sender` | object | `SelfChar` identity, commanders, and sorted `STs` participants. |
//! | `summary` | object | Sender `SOv` and opponent `OOv` aggregate counters. |
//! | `opponents` | array | `Attacks` entries with identity, timing, participants, NPC loot, results, and effects. |
//! | `timeline` | object | Report timestamps, troop samples, and reinforcement event data. |
//!
//! Battle data comes from `body.content`. Metadata adds report ID, role, KvK
//! classification, and optional schema/room fields to the common root metadata.
//! Missing aggregate overviews and per-attack result blocks become objects with
//! null fields, keeping absence distinct from measured zero counters.
//!
//! Attacks are sorted by the numeric prefix of their keys, then by the complete
//! key; the output retains the original key as `attack.id`. Participant keys are
//! sorted as signed integers. Player IDs, slightly wounded counts, and selected
//! power counters also preserve signed values. Timeline samples remain in input
//! order; events without `AssistUnits` are skipped after reading their tick and type.
//!
//! Optional commander, support, auxiliary-skill, and effect data follow the rules
//! in their extraction modules. Missing optional fields do not generally excuse
//! present values of the wrong type.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_battle::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod battle_effects;
mod content;
mod metadata;
mod opponents;
mod participants;
mod player;
mod sender;
mod summary;
mod timeline;

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
        Box::new(sender::SenderExtractor::new()),
        Box::new(summary::SummaryExtractor::new()),
        Box::new(opponents::OpponentsExtractor::new()),
        Box::new(timeline::TimelineExtractor::new()),
    ])
}
